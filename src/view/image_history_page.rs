use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gettextrs::ngettext;
use gtk::CompositeTemplate;
use gtk::glib;
use gtk::glib::clone;
use gtk::pango;

use crate::model;
use crate::rt;
use crate::utils;
use crate::widget;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_history_page.ui")]
    pub(crate) struct ImageHistoryPage {
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) spinner: TemplateChild<adw::Spinner>,
        #[template_child]
        pub(super) preferences_group: TemplateChild<adw::PreferencesGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageHistoryPage {
        const NAME: &'static str = "PdsImageHistoryPage";
        type Type = super::ImageHistoryPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageHistoryPage {
        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ImageHistoryPage {}
}

glib::wrapper! {
    pub(crate) struct ImageHistoryPage(ObjectSubclass<imp::ImageHistoryPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Image> for ImageHistoryPage {
    fn from(image: &model::Image) -> Self {
        let obj = glib::Object::builder::<Self>().build();
        obj.imp()
            .window_title
            .set_subtitle(&utils::format_id(&image.id()));

        rt::Promise::new({
            let api = image.api().unwrap();
            async move { api.history().await }
        })
        .defer(clone!(
            #[weak]
            obj,
            move |result| {
                let imp = obj.imp();

                match result {
                    Ok(entries) => {
                        let len = entries.len() as u32;

                        imp.preferences_group.set_description(Some(&format!(
                            "{}, {}",
                            ngettext!("{} entry", "{} entries", len, len,),
                            gettext!(
                                "{} in total",
                                glib::format_size(
                                    entries
                                        .iter()
                                        .filter_map(|item| item.size)
                                        .map(|size| size as u64)
                                        .sum()
                                ),
                            )
                        )));

                        entries.into_iter().for_each(|entry| {
                            let row = adw::ExpanderRow::builder()
                                .title(
                                    entry
                                        .id
                                        .as_deref()
                                        .map(utils::format_id)
                                        .unwrap_or_else(|| gettext("<None>")),
                                )
                                .subtitle(
                                    entry
                                        .created
                                        .map(|created| {
                                            glib::DateTime::from_unix_local(created)
                                                .unwrap()
                                                .format(
                                                    // Translators: This is a date time format (https://valadoc.org/glib-2.0/GLib.DateTime.format.html)
                                                    &gettext("%x %X"),
                                                )
                                                .unwrap()
                                                .to_string()
                                        })
                                        .unwrap_or_else(|| gettext("Unknown Date")),
                                )
                                .use_markup(false)
                                .build();

                            row.add_action(
                                &gtk::Label::builder()
                                    .label(
                                        entry
                                            .size
                                            .map(|size| {
                                                String::from(glib::format_size(size as u64))
                                            })
                                            .unwrap_or_else(|| gettext("Unknown size")),
                                    )
                                    .css_classes(vec!["dim-label".to_string()])
                                    .build(),
                            );

                            if let Some(created_by) = entry.created_by {
                                let box_ = gtk::Box::builder()
                                    .orientation(gtk::Orientation::Vertical)
                                    .spacing(9)
                                    .margin_top(9)
                                    .margin_end(12)
                                    .margin_bottom(9)
                                    .margin_start(12)
                                    .build();
                                box_.append(
                                    &gtk::Label::builder()
                                        .label(gettext("Created by"))
                                        .xalign(0.0)
                                        .css_classes(vec!["heading".to_string()])
                                        .build(),
                                );
                                box_.append(
                                    &gtk::Label::builder()
                                        .label(created_by)
                                        .single_line_mode(false)
                                        .xalign(0.0)
                                        .wrap(true)
                                        .wrap_mode(pango::WrapMode::WordChar)
                                        .selectable(true)
                                        .build(),
                                );

                                row.add_row(
                                    &adw::PreferencesRow::builder()
                                        .activatable(false)
                                        .child(&box_)
                                        .build(),
                                );
                            }
                            if let Some(comment) = entry.comment {
                                if !comment.is_empty() {
                                    row.add_row(&property_row(&gettext("Comment"), &comment));
                                }
                            }
                            if let Some(tags) = entry.tags {
                                row.add_row(&property_row(&gettext("Tags"), &tags.join(", ")));
                            }

                            imp.preferences_group.add(&row);
                        });

                        imp.stack.set_visible_child_name("loaded");
                    }
                    Err(e) => {
                        log::error!("Error on retrieving history: {e}");

                        imp.spinner.set_visible(false);
                        utils::show_error_toast(
                            &obj,
                            &gettext("Error on retrieving history"),
                            &e.to_string(),
                        );
                    }
                }
            }
        ));

        obj
    }
}

fn property_row(key: &str, value: &str) -> widget::PropertyRow {
    let row = widget::PropertyRow::new(key, value);
    row.set_activatable(false);
    row
}
