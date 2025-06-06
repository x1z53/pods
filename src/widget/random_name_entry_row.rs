use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/random_name_entry_row.ui")]
    pub(crate) struct RandomNameEntryRow {
        pub(super) names: RefCell<names::Generator<'static>>,
        #[template_child]
        pub(super) generate_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RandomNameEntryRow {
        const NAME: &'static str = "PdsRandomNameEntryRow";
        type Type = super::RandomNameEntryRow;
        type ParentType = adw::EntryRow;
        type Interfaces = (gtk::Editable,);

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(
                "random-name-entry-row.generate",
                None,
                move |widget, _, _| {
                    widget.generate_random_name();
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RandomNameEntryRow {
        fn set_property(&self, _: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "text" => self.obj().set_text(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "text" => self.obj().text().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().generate_random_name()
        }
    }

    impl WidgetImpl for RandomNameEntryRow {}
    impl ListBoxRowImpl for RandomNameEntryRow {}
    impl PreferencesRowImpl for RandomNameEntryRow {}
    impl EntryRowImpl for RandomNameEntryRow {}
    impl EditableImpl for RandomNameEntryRow {}
}

glib::wrapper! {
    pub(crate) struct RandomNameEntryRow(ObjectSubclass<imp::RandomNameEntryRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::EntryRow,
        @implements gtk::Editable;
}

impl Default for RandomNameEntryRow {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl RandomNameEntryRow {
    pub(crate) fn generate_random_name(&self) {
        self.set_text(&self.imp().names.borrow_mut().next().unwrap());
    }
}
