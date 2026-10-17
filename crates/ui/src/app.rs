//! The interface root: the navigation, the frame, and the dispatcher.
//!
//! [`Ui`] holds only what is a matter of *what is on screen* — which tab, what has
//! been typed, what is selected. Everything else arrives in a [`World`] and leaves
//! as an [`Action`], so this file never reads a config file, never makes a request
//! and never starts a process.
//!
//! egui 0.36 lays panels out *inside a `Ui`* rather than against a `Context`, which
//! is why [`Ui::show`] takes one: the shell's `eframe::App::ui` hands it the root.

use egui::{Color32, CornerRadius, Margin, Rect, RichText, Sense, pos2, vec2};

use crate::icons::{self, Icon};
use crate::model::{Action, LibraryView, ProbeReport, Tab, World};
use crate::theme::{
    Palette, RADIUS_MD, SIDEBAR_WIDTH, SPACE_LG, SPACE_MD, SPACE_SM, SPACE_XL, SPACE_XS, TOPBAR_HEIGHT,
    Theme, ThemeChoice,
};
use crate::views;
use crate::widgets::{self, ButtonKind};

/// The interface's own state.
#[derive(Debug, Clone)]
pub struct Ui {
    /// The destination currently shown.
    pub tab: Tab,
    /// The search text.
    pub search: String,
    /// The app whose detail page is open.
    pub selected: Option<String>,
    /// How the library is laid out.
    pub view: LibraryView,
    /// The app whose removal is being confirmed.
    pub confirm_remove: Option<String>,
    /// The theme the person chose.
    pub theme_choice: ThemeChoice,
    /// Whether pre-releases are offered.
    pub prerelease: bool,
    /// What has been typed into the add-repository field.
    pub add_input: String,
    /// Where the app being added already lives on this machine.
    pub local_input: String,
    /// What the last probe of a repository found.
    pub probe: Option<ProbeReport>,
    /// Whether a probe is in flight.
    pub probing: bool,
    /// Whether the About panel is open.
    pub show_about: bool,
    /// Whether the check-for-updates confirmation is open.
    pub confirm_check: bool,
    /// The theme last installed, so the style is not rebuilt every frame.
    applied: Option<(ThemeChoice, bool)>,
}

impl Ui {
    /// A fresh interface.
    #[must_use]
    pub fn new(theme_choice: ThemeChoice, prerelease: bool, view: LibraryView) -> Self {
        Self {
            tab: Tab::Library,
            search: String::new(),
            selected: None,
            view,
            confirm_remove: None,
            theme_choice,
            prerelease,
            add_input: String::new(),
            local_input: String::new(),
            probe: None,
            probing: false,
            show_about: false,
            confirm_check: false,
            applied: None,
        }
    }

    /// Draw a frame and collect what was asked for.
    pub fn show(&mut self, root: &mut egui::Ui, world: &World) -> Vec<Action> {
        let mut actions = Vec::new();
        let ctx = root.ctx().clone();
        let theme = self.sync_theme(&ctx);

        // The rail is added first so it runs the full height of the window, with the
        // content's own bar beside it rather than above it.
        egui::Panel::left("vnl-nav")
            .exact_size(SIDEBAR_WIDTH)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(theme.palette.bg_panel)
                    .inner_margin(Margin::same(SPACE_MD as i8)),
            )
            .show(root, |ui| self.rail(ui, &theme, world, &mut actions));

        egui::Panel::top("vnl-top")
            .exact_size(TOPBAR_HEIGHT)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(theme.palette.bg_base)
                    .inner_margin(Margin::symmetric(SPACE_XL as i8, 0)),
            )
            .show(root, |ui| self.top_bar(ui, &theme, world, &mut actions));

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme.palette.bg_base)
                    .inner_margin(Margin {
                        left: SPACE_XL as i8,
                        right: SPACE_XL as i8,
                        top: SPACE_LG as i8,
                        bottom: SPACE_XL as i8,
                    }),
            )
            .show(root, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.notices(ui, &theme, world, &mut actions);
                        self.content(ui, &theme, world, &mut actions);
                    });
            });

        if self.show_about {
            self.about_window(&ctx, &theme, &mut actions);
        }
        if self.confirm_check {
            self.confirm_window(&ctx, &theme, world, &mut actions);
        }
        if self.confirm_remove.is_some() {
            self.remove_window(&ctx, &theme, world, &mut actions);
        }

        actions
    }

    /// Install the theme when the choice or the system's preference has changed.
    ///
    /// Only the style is (re)applied here. The typefaces are registered once, before
    /// the first frame, by the shell: `set_fonts` lands on the next pass, and text
    /// drawn with a family that is not bound yet panics.
    fn sync_theme(&mut self, ctx: &egui::Context) -> Theme {
        let system_dark = !matches!(ctx.system_theme(), Some(egui::Theme::Light));
        let theme = Theme::resolve(self.theme_choice, system_dark);
        if self.applied != Some((self.theme_choice, system_dark)) {
            theme.apply(ctx);
            self.applied = Some((self.theme_choice, system_dark));
        }
        theme
    }

    /// The navigation rail.
    fn rail(&mut self, ui: &mut egui::Ui, theme: &Theme, world: &World, actions: &mut Vec<Action>) {
        ui.add_space(SPACE_XS);
        self.wordmark(ui, theme);
        ui.add_space(SPACE_XL);

        let attention = world.attention_count();
        for tab in Tab::ALL {
            let count = match tab {
                Tab::Library => Some(world.apps.len()).filter(|n| *n > 0),
                Tab::Updates => Some(attention).filter(|n| *n > 0),
                Tab::Discover | Tab::Settings => None,
            };
            self.rail_item(ui, theme, tab, count, tab == self.tab, actions);
            ui.add_space(2.0);
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(SPACE_XS);
            if widgets::button_full(ui, theme, ButtonKind::Ghost, Some(Icon::Info), "About", true).clicked() {
                self.show_about = true;
            }
            ui.add_space(SPACE_SM);
            ui.label(
                RichText::new(format!("vn-launcher {}", vnl_core::VERSION))
                    .font(theme.text.small.clone())
                    .color(theme.palette.text_tertiary),
            );
            ui.add_space(SPACE_XS);
            let summary = if world.busy {
                "checking…".to_owned()
            } else {
                match world.checked_at {
                    Some(at) => format!(
                        "{} apps · checked {}",
                        world.apps.len(),
                        vnl_core::time::describe_age(at, now_unix())
                    ),
                    None => format!("{} apps · not checked yet", world.apps.len()),
                }
            };
            ui.label(
                RichText::new(summary)
                    .font(theme.text.micro.clone())
                    .color(theme.palette.text_tertiary),
            );
        });
    }

    /// The product mark at the top of the rail.
    ///
    /// The mark itself, in the interface's own ink — not a coloured tile. There is one
    /// mark now: this is the same geometry `assets/logo/` ships and the window icon
    /// carries, and a second, blue treatment of an older glyph in the corner of the
    /// window was the previous identity lingering after the rest had moved on.
    fn wordmark(&self, ui: &mut egui::Ui, theme: &Theme) {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
            // `Icon::Crystal` scales its own stroke from the box it is given, so there is
            // no width to pass. See the variant's own documentation.
            icons::paint(ui.painter(), Icon::Crystal, rect, theme.palette.text_primary, 0.0);
            ui.add_space(SPACE_SM);
            ui.label(
                RichText::new("vn-launcher")
                    .font(theme.text.heading.clone())
                    .color(theme.palette.text_primary),
            );
        });
    }

    /// One destination in the rail.
    fn rail_item(
        &self,
        ui: &mut egui::Ui,
        theme: &Theme,
        tab: Tab,
        count: Option<usize>,
        selected: bool,
        actions: &mut Vec<Action>,
    ) {
        let height = 36.0;
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());

        if ui.is_rect_visible(rect) {
            let fill = if selected {
                Palette::tint(theme.palette.accent, if theme.dark { 46 } else { 30 })
            } else if response.hovered() {
                theme.palette.bg_hover
            } else {
                Color32::TRANSPARENT
            };
            if fill != Color32::TRANSPARENT {
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(RADIUS_MD), fill);
            }

            let foreground = if selected {
                theme.palette.accent
            } else {
                theme.palette.text_secondary
            };

            let mut left = rect.left() + SPACE_MD;
            let icon_rect = Rect::from_min_size(pos2(left, rect.center().y - 8.0), vec2(16.0, 16.0));
            icons::paint(ui.painter(), tab.icon(), icon_rect, foreground, 1.6);
            left += 16.0 + SPACE_SM;

            let galley =
                ui.painter()
                    .layout_no_wrap(tab.label().to_owned(), theme.text.label.clone(), foreground);
            let text_width = galley.size().x;
            let text_y = rect.center().y - galley.size().y / 2.0;
            ui.painter().galley(pos2(left, text_y), galley, foreground);

            if let Some(count) = count {
                let badge = ui.painter().layout_no_wrap(
                    count.to_string(),
                    theme.text.micro.clone(),
                    theme.palette.text_secondary,
                );
                let width = badge.size().x.max(8.0) + SPACE_SM;
                let badge_rect = Rect::from_min_size(
                    pos2(rect.right() - SPACE_MD - width, rect.center().y - 9.0),
                    vec2(width, 18.0),
                );
                if badge_rect.left() > left + text_width + SPACE_SM {
                    ui.painter()
                        .rect_filled(badge_rect, CornerRadius::same(9), theme.palette.bg_active);
                    ui.painter().galley(
                        pos2(
                            badge_rect.center().x - badge.size().x / 2.0,
                            badge_rect.center().y - badge.size().y / 2.0,
                        ),
                        badge,
                        theme.palette.text_secondary,
                    );
                }
            }
        }

        if response.clicked() {
            actions.push(Action::SetTab(tab));
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }

    /// The bar above the content.
    fn top_bar(&mut self, ui: &mut egui::Ui, theme: &Theme, world: &World, actions: &mut Vec<Action>) {
        ui.horizontal_centered(|ui| {
            let title = match (&self.tab, &self.selected) {
                (Tab::Library, Some(id)) => world
                    .app(id)
                    .map_or_else(|| "Library".to_owned(), |card| card.entry.name.clone()),
                (tab, _) => tab.label().to_owned(),
            };
            ui.label(
                RichText::new(title)
                    .font(theme.text.title.clone())
                    .color(theme.palette.text_primary),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let refresh = widgets::button(
                    ui,
                    theme,
                    ButtonKind::Secondary,
                    Some(Icon::Refresh),
                    "Check for updates",
                    !world.busy,
                );
                if refresh.clicked() {
                    // Reading every repository is one request each against an anonymous
                    // rate limit, so it asks first rather than spending them on a click
                    // that was only meant to look.
                    self.confirm_check = true;
                }

                if world.busy {
                    ui.add_space(SPACE_SM);
                    ui.spinner();
                }

                if matches!(self.tab, Tab::Library | Tab::Updates) {
                    ui.add_space(SPACE_MD);
                    let search = egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Search")
                        .desired_width(240.0)
                        .font(theme.text.body.clone())
                        .margin(Margin::symmetric(SPACE_SM as i8, 6));
                    ui.add(search);
                }

                // The layout applies to the library itself, so it is offered beside the
                // library and not on the detail page, where a card or a row is not what is
                // on screen. Right-to-left, so this lands between the search field and the
                // check button.
                if self.tab == Tab::Library && self.selected.is_none() {
                    ui.add_space(SPACE_SM);
                    for view in LibraryView::ALL.into_iter().rev() {
                        let selected = self.view == view;
                        let response =
                            widgets::icon_toggle(ui, theme, view.icon(), selected, view.label(), true);
                        if response.clicked() && !selected {
                            self.view = view;
                            actions.push(Action::SetLibraryView(view));
                        }
                    }
                }
            });
        });
    }

    /// Messages from the shell.
    fn notices(&mut self, ui: &mut egui::Ui, theme: &Theme, world: &World, actions: &mut Vec<Action>) {
        if world.notices.is_empty() {
            return;
        }
        for (index, notice) in world.notices.iter().enumerate() {
            if widgets::notice_banner(ui, theme, notice).clicked() {
                actions.push(Action::DismissNotice(index));
            }
            ui.add_space(SPACE_SM);
        }
        ui.add_space(SPACE_XS);
    }

    /// The view for the current destination.
    fn content(&mut self, ui: &mut egui::Ui, theme: &Theme, world: &World, actions: &mut Vec<Action>) {
        match self.tab {
            Tab::Library => match self.selected.clone() {
                Some(id) if world.app(&id).is_some() => {
                    views::detail::show(ui, theme, world, self, actions);
                }
                Some(_) => {
                    self.selected = None;
                    views::library::show(ui, theme, world, self, actions);
                }
                None => views::library::show(ui, theme, world, self, actions),
            },
            Tab::Updates => views::updates::show(ui, theme, world, self, actions),
            Tab::Discover => views::discover::show(ui, theme, world, self, actions),
            Tab::Settings => views::settings::show(ui, theme, world, self, actions),
        }
    }

    /// A modal with the version and where things live.
    ///
    /// Shown against the `Context` rather than the root `Ui`, because a window is an
    /// area of its own rather than a region of the panels.
    fn about_window(&mut self, ctx: &egui::Context, theme: &Theme, actions: &mut Vec<Action>) {
        let mut open = true;
        let frame = egui::Frame::new()
            .fill(theme.palette.bg_elevated)
            .stroke(egui::Stroke::new(1.0, theme.palette.border_subtle))
            .corner_radius(CornerRadius::same(crate::theme::RADIUS_LG))
            .inner_margin(Margin::same(SPACE_XL as i8));

        egui::Window::new("About vn-launcher")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_max_width(400.0);

                // The mark, centred above the name.
                ui.vertical_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::hover());
                    icons::paint(ui.painter(), Icon::Crystal, rect, theme.palette.text_primary, 0.0);
                });
                ui.add_space(SPACE_MD);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!("vn-launcher {}", vnl_core::VERSION))
                            .font(theme.text.title.clone())
                            .color(theme.palette.text_primary),
                    );
                });
                ui.add_space(SPACE_SM);
                ui.label(
                    RichText::new(
                        "Tracks the GitHub repositories you add, reads their public releases, \
                         and starts what is already installed. No account, no token, no sign-in.",
                    )
                    .font(theme.text.small.clone())
                    .color(theme.palette.text_secondary),
                );
                ui.add_space(SPACE_LG);
                if widgets::button_full(
                    ui,
                    theme,
                    ButtonKind::Secondary,
                    Some(Icon::External),
                    "Repository",
                    true,
                )
                .clicked()
                {
                    actions.push(Action::OpenUrl(format!(
                        "https://github.com/{}",
                        crate::LAUNCHER_REPO
                    )));
                }
                ui.add_space(SPACE_SM);
                if widgets::button_full(ui, theme, ButtonKind::Ghost, Some(Icon::Close), "Close", true)
                    .clicked()
                {
                    self.show_about = false;
                }
            });

        if !open {
            self.show_about = false;
        }
    }

    /// Ask before reading every repository.
    ///
    /// The top-bar button used to refresh on the spot. A check is not free — it is one
    /// request per repository against an anonymous rate limit of sixty an hour — so it
    /// asks first, and says what pressing yes will spend.
    fn confirm_window(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        world: &World,
        actions: &mut Vec<Action>,
    ) {
        let mut open = self.confirm_check;
        let frame = egui::Frame::new()
            .fill(theme.palette.bg_elevated)
            .stroke(egui::Stroke::new(1.0, theme.palette.border_subtle))
            .corner_radius(CornerRadius::same(crate::theme::RADIUS_LG))
            .inner_margin(Margin::same(SPACE_XL as i8));

        let count = world.apps.len();
        egui::Window::new("Check for updates")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_max_width(380.0);
                ui.label(
                    RichText::new(match count {
                        0 => "There are no repositories to read yet.".to_owned(),
                        1 => "Read the public releases of the one repository you track?".to_owned(),
                        n => format!("Read the public releases of all {n} repositories?"),
                    })
                    .font(theme.text.body.clone())
                    .color(theme.palette.text_primary),
                );
                ui.add_space(SPACE_SM);
                ui.label(
                    RichText::new(if count == 0 {
                        "Add one from the Add repository tab first."
                    } else {
                        "That is one request per repository to GitHub's public API, as an \
                         anonymous guest. Answers are kept, so a second check in a row does \
                         not spend a second request."
                    })
                    .font(theme.text.small.clone())
                    .color(theme.palette.text_secondary),
                );
                ui.add_space(SPACE_LG);

                ui.columns(2, |columns| {
                    let yes = widgets::button_full(
                        &mut columns[0],
                        theme,
                        ButtonKind::Primary,
                        Some(Icon::Refresh),
                        "Yes, check",
                        count > 0,
                    );
                    if yes.clicked() {
                        actions.push(Action::Refresh);
                        self.confirm_check = false;
                    }

                    let no = widgets::button_full(
                        &mut columns[1],
                        theme,
                        // Secondary rather than Ghost: in a two-button question an
                        // unbordered "No" reads as a label, not as something to press.
                        ButtonKind::Secondary,
                        Some(Icon::Close),
                        "No",
                        true,
                    );
                    if no.clicked() {
                        self.confirm_check = false;
                    }
                });
            });

        // The title bar's own close button flips `open` rather than calling back, so
        // without this the window would close for one frame and return the next.
        if !open {
            self.confirm_check = false;
        }
    }

    /// Ask before taking an app out of the launcher.
    ///
    /// The dialog states both halves of what removal is — the entry, and the files —
    /// because they have different consequences. The entry can be added back from the
    /// Add repository tab; the deleted copy cannot, so it is named before it goes.
    fn remove_window(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        world: &World,
        actions: &mut Vec<Action>,
    ) {
        let Some(id) = self.confirm_remove.clone() else {
            return;
        };
        let Some(card) = world.app(&id) else {
            // The app is already gone — the card was removed by something else between
            // the click and this frame. A dialog about nothing is worse than none.
            self.confirm_remove = None;
            return;
        };

        let name = card.entry.name.clone();
        let install_dir = format!("{}/{id}", world.install_root);
        let found = matches!(card.status, vnl_core::AppStatus::Found { .. });

        let mut open = true;
        let frame = egui::Frame::new()
            .fill(theme.palette.bg_elevated)
            .stroke(egui::Stroke::new(1.0, theme.palette.border_subtle))
            .corner_radius(CornerRadius::same(crate::theme::RADIUS_LG))
            .inner_margin(Margin::same(SPACE_XL as i8));

        egui::Window::new(format!("Remove {name}?"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_max_width(440.0);
                ui.label(
                    RichText::new(format!(
                        "The launcher will stop tracking {name}, and will not check \
                         {} again.",
                        card.entry.repo.slug()
                    ))
                    .font(theme.text.body.clone())
                    .color(theme.palette.text_primary),
                );

                ui.add_space(SPACE_MD);
                widgets::field(ui, theme, "Files it would delete", &install_dir, true);
                ui.add_space(SPACE_XS);
                ui.label(
                    RichText::new(if found {
                        format!(
                            "{name} is on this machine, but the launcher did not install it, \
                             so nothing outside the folder above is touched — and that folder \
                             is not where it was found."
                        )
                    } else {
                        "Only what the launcher put in that folder. An app found on PATH, in a \
                         folder you named, or in a clone is never deleted — the launcher did \
                         not put it there."
                            .to_owned()
                    })
                    .font(theme.text.small.clone())
                    .color(theme.palette.text_secondary),
                );

                ui.add_space(SPACE_LG);
                ui.columns(2, |columns| {
                    let remove = widgets::button_full(
                        &mut columns[0],
                        theme,
                        ButtonKind::Danger,
                        Some(Icon::Trash),
                        "Remove it",
                        true,
                    );
                    if remove.clicked() {
                        actions.push(Action::RemoveApp(id.clone()));
                        self.confirm_remove = None;
                    }

                    let keep = widgets::button_full(
                        &mut columns[1],
                        theme,
                        ButtonKind::Secondary,
                        Some(Icon::Close),
                        "Keep it",
                        true,
                    );
                    if keep.clicked() {
                        self.confirm_remove = None;
                    }
                });
            });

        if !open {
            self.confirm_remove = None;
        }
    }
}

/// Seconds since the Unix epoch.
#[must_use]
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}
