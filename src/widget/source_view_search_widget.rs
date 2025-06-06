use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use sourceview5::prelude::*;

use crate::utils;
use crate::widget;

const ACTION_SEARCH_BACKWARDS: &str = "source-view-search-widget.search-backward";
const ACTION_SEARCH_FORWARD: &str = "source-view-search-widget.search-forward";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::SourceViewSearchWidget)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/source_view_search_widget.ui")]
    pub(crate) struct SourceViewSearchWidget {
        pub(super) search_settings: sourceview5::SearchSettings,
        pub(super) search_context: RefCell<Option<sourceview5::SearchContext>>,
        pub(super) search_iters: RefCell<Option<(gtk::TextIter, gtk::TextIter)>>,
        #[property(get, set = Self::set_source_view, nullable)]
        pub(super) source_view: glib::WeakRef<sourceview5::View>,
        #[template_child]
        pub(super) search_entry: TemplateChild<widget::TextSearchEntry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SourceViewSearchWidget {
        const NAME: &'static str = "PdsSourceViewSearchWidget";
        type Type = super::SourceViewSearchWidget;
        type ParentType = gtk::Widget;
        type Interfaces = (gtk::Editable,);

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.add_binding_action(
                gdk::Key::G,
                gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
                ACTION_SEARCH_BACKWARDS,
            );
            klass.install_action(ACTION_SEARCH_BACKWARDS, None, |widget, _, _| {
                widget.search_backward();
            });

            klass.add_binding_action(
                gdk::Key::G,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_SEARCH_FORWARD,
            );
            klass.install_action(ACTION_SEARCH_FORWARD, None, |widget, _, _| {
                widget.search_forward();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SourceViewSearchWidget {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let name = pspec.name();
            if self.search_entry.has_property(name, None) {
                self.search_entry.set_property(name, value);
            } else {
                self.derived_set_property(id, value, pspec);
            }
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let name = pspec.name();
            if self.search_entry.has_property(name, None) {
                self.search_entry.property(name)
            } else {
                self.derived_property(id, pspec)
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            self.search_entry
                .bind_property("text", &self.search_settings, "search-text")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.search_settings.set_wrap_around(true);

            self.search_entry
                .bind_property("regex", &self.search_settings, "regex-enabled")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.search_entry
                .bind_property("case-sensitive", &self.search_settings, "case-sensitive")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.search_entry
                .bind_property("whole-word", &self.search_settings, "at-word-boundaries")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for SourceViewSearchWidget {
        fn grab_focus(&self) -> bool {
            self.search_entry.grab_focus()
        }
    }

    impl EditableImpl for SourceViewSearchWidget {
        fn delegate(&self) -> Option<gtk::Editable> {
            Some(self.search_entry.clone().upcast())
        }
    }

    impl SourceViewSearchWidget {
        pub(super) fn set_source_view(&self, value: Option<&sourceview5::View>) {
            let obj = &*self.obj();

            if obj.source_view().as_ref() == value {
                return;
            }

            if let Some(source_view) = value {
                let search_context = sourceview5::SearchContext::new(
                    source_view
                        .buffer()
                        .downcast_ref::<sourceview5::Buffer>()
                        .unwrap(),
                    Some(&self.search_settings),
                );

                search_context.connect_occurrences_count_notify(clone!(
                    #[weak]
                    obj,
                    move |ctx| {
                        obj.imp()
                            .search_entry
                            .set_info(&gettext!("0 of {}", ctx.occurrences_count(),));
                    }
                ));

                self.search_context.replace(Some(search_context));
            }

            self.source_view.set(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct SourceViewSearchWidget(ObjectSubclass<imp::SourceViewSearchWidget>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Editable;
}

impl SourceViewSearchWidget {
    fn update_search_occurrences(&self) {
        let imp = self.imp();

        if let Some(search_context) = imp.search_context.borrow().as_ref() {
            let count = search_context.occurrences_count();
            imp.search_entry.set_info(&if count > 0 {
                gettext!(
                    "{} of {}",
                    imp.search_iters
                        .borrow()
                        .as_ref()
                        .map(|(start_iter, end_iter)| search_context
                            .occurrence_position(start_iter, end_iter))
                        .unwrap_or(0),
                    count
                )
            } else {
                String::new()
            });
        }
    }

    pub(crate) fn search_backward(&self) {
        if let Some(source_view) = self.source_view() {
            let source_buffer = source_view.buffer();
            let imp = self.imp();

            let iter_at_cursor = source_buffer.iter_at_offset({
                let pos = source_buffer.cursor_position();
                if pos >= 0 { pos } else { i32::MAX }
            });

            imp.search_iters.replace_with(|iters| {
                match imp.search_context.borrow().as_ref().unwrap().backward(
                    &iters
                        .map(|(start_iter, end_iter)| {
                            if iter_at_cursor >= start_iter && iter_at_cursor <= end_iter {
                                start_iter
                            } else {
                                iter_at_cursor
                            }
                        })
                        .unwrap_or(iter_at_cursor),
                ) {
                    Some((mut start, end, _)) => {
                        source_view.scroll_to_iter(&mut start, 0.0, false, 0.0, 0.0);
                        source_buffer.place_cursor(&start);

                        Some((start, end))
                    }
                    None => None,
                }
            });

            self.update_search_occurrences();
        }
    }

    pub(crate) fn search_forward(&self) {
        if let Some(source_view) = self.source_view() {
            let source_buffer = source_view.buffer();
            let imp = self.imp();

            let iter_at_cursor = source_buffer.iter_at_offset({
                let pos = source_buffer.cursor_position();
                if pos > 0 { pos } else { 0 }
            });

            imp.search_iters.replace_with(|iters| {
                match imp.search_context.borrow().as_ref().unwrap().forward(
                    &iters
                        .map(|(start_iter, end_iter)| {
                            if iter_at_cursor >= start_iter && iter_at_cursor <= end_iter {
                                end_iter
                            } else {
                                iter_at_cursor
                            }
                        })
                        .unwrap_or(iter_at_cursor),
                ) {
                    Some((start, mut end, _)) => {
                        source_view.scroll_to_iter(&mut end, 0.0, false, 0.0, 0.0);
                        source_buffer.place_cursor(&end);

                        Some((start, end))
                    }
                    None => None,
                }
            });

            self.update_search_occurrences();
        }
    }
}
