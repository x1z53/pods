use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::Deref;
use std::sync::OnceLock;

use glib::Properties;
use glib::clone;
use glib::prelude::*;
use glib::subclass::Signal;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::model;
use crate::podman;
use crate::rt;

mod imp {
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::Image)]
    pub(crate) struct Image {
        pub(super) inspection_callbacks: RefCell<
            Option<rt::Callbacks<podman::Result<podman::models::InspectImageResponseLibpod>>>,
        >,
        #[property(get, set, construct_only, nullable)]
        pub(super) image_list: glib::WeakRef<model::ImageList>,
        #[property(get = Self::container_list)]
        pub(super) container_list: OnceCell<model::SimpleContainerList>,
        #[property(get)]
        pub(super) containers: Cell<u64>,
        #[property(get, set, construct_only)]
        pub(super) created: OnceCell<i64>,
        #[property(get)]
        pub(super) dangling: Cell<bool>,
        #[property(get = Self::data, nullable)]
        pub(super) data: OnceCell<Option<model::ImageData>>,
        #[property(get, set, construct_only)]
        pub(super) id: OnceCell<String>,
        #[property(get = Self::repo_tags)]
        pub(super) repo_tags: OnceCell<model::RepoTagList>,
        #[property(get, set, construct_only)]
        pub(super) size: OnceCell<u64>,
        #[property(get)]
        pub(super) shared_size: Cell<u64>,
        #[property(get)]
        pub(super) virtual_size: Cell<u64>,
        #[property(get)]
        pub(super) to_be_deleted: Cell<bool>,
        #[property(get, set)]
        pub(super) selected: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Image {
        const NAME: &'static str = "Image";
        type Type = super::Image;
        type Interfaces = (model::Selectable,);
    }

    impl ObjectImpl for Image {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("deleted").build()])
        }

        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }

    impl Image {
        pub(super) fn container_list(&self) -> model::SimpleContainerList {
            self.container_list.get_or_init(Default::default).to_owned()
        }

        pub(super) fn data(&self) -> Option<model::ImageData> {
            self.data.get().cloned().flatten()
        }

        pub(super) fn set_data(&self, value: model::ImageData) {
            if self.data().is_some() {
                return;
            }
            self.data.set(Some(value)).unwrap();
            self.obj().notify_data();
        }

        pub(super) fn set_containers(&self, value: u64) {
            let obj = &*self.obj();
            if obj.containers() == value {
                return;
            }
            self.containers.set(value);
            obj.notify_containers();
        }

        pub(super) fn set_dangling(&self, value: bool) {
            let obj = &*self.obj();
            if obj.dangling() == value {
                return;
            }
            self.dangling.set(value);
            obj.notify_dangling();
        }

        pub(super) fn repo_tags(&self) -> model::RepoTagList {
            self.repo_tags
                .get_or_init(|| model::RepoTagList::from(&*self.obj()))
                .to_owned()
        }

        pub(super) fn set_shared_size(&self, value: u64) {
            let obj = &*self.obj();
            if obj.shared_size() == value {
                return;
            }
            self.shared_size.set(value);
            obj.notify_shared_size();
        }

        pub(super) fn set_virtual_size(&self, value: u64) {
            let obj = &*self.obj();
            if obj.virtual_size() == value {
                return;
            }
            self.virtual_size.set(value);
            obj.notify_virtual_size();
        }

        pub(super) fn set_to_be_deleted(&self, value: bool) {
            let obj = &*self.obj();
            if obj.to_be_deleted() == value {
                return;
            }
            self.to_be_deleted.set(value);
            obj.notify_to_be_deleted();
        }
    }
}

glib::wrapper! {
    pub(crate) struct Image(ObjectSubclass<imp::Image>) @implements model::Selectable;
}

impl Image {
    pub(crate) fn new(
        image_list: &model::ImageList,
        summary: &podman::models::LibpodImageSummary,
    ) -> Self {
        glib::Object::builder::<Self>()
            .property("image-list", image_list)
            .property("created", summary.created.unwrap_or(0))
            .property("id", &summary.id)
            .property("size", summary.size.unwrap_or_default() as u64)
            .build()
            .update_internal(summary, false)
            .to_owned()
    }

    fn update_internal(
        &self,
        summary: &podman::models::LibpodImageSummary,
        notify_repo_tags: bool,
    ) -> &Self {
        let imp = self.imp();

        imp.set_containers(summary.containers.unwrap_or_default() as u64);
        imp.set_dangling(summary.dangling.unwrap_or_default());
        if self.repo_tags().update(HashSet::from_iter(
            summary.repo_tags.as_deref().unwrap_or_default().iter(),
        )) && notify_repo_tags
        {
            if let Some(list) = self.image_list() {
                list.notify("intermediates");
            }
        }
        imp.set_shared_size(summary.shared_size.unwrap_or_default() as u64);
        imp.set_virtual_size(summary.virtual_size.unwrap_or_default() as u64);

        self
    }

    pub(crate) fn update(&self, summary: &podman::models::LibpodImageSummary) -> &Self {
        self.update_internal(summary, true)
    }

    pub(crate) fn inspect<F>(&self, op: F)
    where
        F: Fn(Result<model::Image, &podman::Error>) + 'static,
    {
        if let Some(callbacks) = self.imp().inspection_callbacks.borrow().as_ref() {
            callbacks.add(clone!(
                #[weak(rename_to = obj)]
                self,
                move |result| match result {
                    Ok(_) => op(Ok(obj)),
                    Err(e) => {
                        log::error!("Error on inspecting image '{}': {e}", obj.id());
                        op(Err(e));
                    }
                }
            ));

            return;
        }

        let callbacks = rt::Promise::new({
            let image = self.api().unwrap();
            async move { image.inspect().await }
        })
        .defer_with_callbacks(clone!(
            #[weak(rename_to = obj)]
            self,
            move |result| {
                let imp = obj.imp();

                imp.inspection_callbacks.replace(None);

                match result {
                    Ok(data) => {
                        imp.set_data(model::ImageData::from(data));
                        op(Ok(obj));
                    }
                    Err(e) => {
                        log::error!("Error on inspecting image '{}': {e}", obj.id());
                        op(Err(e));
                    }
                }
            }
        ));

        self.imp().inspection_callbacks.replace(Some(callbacks));
    }
}

impl Image {
    pub(crate) fn delete<F>(&self, op: F)
    where
        F: FnOnce(&Self, podman::Result<()>) + 'static,
    {
        let image = if let Some(image) = self.api() {
            image
        } else {
            return;
        };

        self.imp().set_to_be_deleted(true);

        rt::Promise::new(async move { image.remove().await }).defer(clone!(
            #[weak(rename_to = obj)]
            self,
            move |result| {
                if let Err(ref e) = result {
                    obj.imp().set_to_be_deleted(false);
                    log::error!("Error on removing image: {}", e);
                }
                op(&obj, result);
            }
        ));
    }

    pub(super) fn emit_deleted(&self) {
        self.emit_by_name::<()>("deleted", &[]);
    }

    pub(crate) fn connect_deleted<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("deleted", true, move |values| {
            f(&values[0].get::<Self>().unwrap());

            None
        })
    }

    pub(crate) fn api(&self) -> Option<podman::api::Image> {
        self.image_list()
            .unwrap()
            .client()
            .map(|client| podman::api::Image::new(client.podman().deref().clone(), self.id()))
    }
}
