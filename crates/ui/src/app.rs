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
use crate::model::{Action, ProbeReport, Tab, World};
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
    /// The theme the person chose.
    pub theme_choice: ThemeChoice,
    /// How often updates are looked for, in hours.
    pub interval_hours: u32,
    /// Whether pre-releases are offered.
    pub prerelease: bool,
    /// What has been typed into the add-repository field.
    pub add_input: String,
    /// What the last probe of a repository found.
    pub probe: Option<ProbeReport>,
    /// Whether a probe is in flight.
    pub probing: bool,
    /// Whether the About panel is open.
    pub show_about: bool,
    /// The theme last installed, so the style is not rebuilt every frame.
    applied: Option<(ThemeChoice, bool)>,
}

impl Ui {
    /// A fresh interface.
    #[must_use]
    pub fn new(theme_choice: ThemeChoice, interval_hours: u32, prerelease: bool) -> Self {
        Self {
            tab: Tab::Library,
            search: String::new(),
            selected: None,
            theme_choice,
            interval_hours,
            prerelease,
            add_input: String::new(),
            probe: None,
            probing: false,
            show_about: false,
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
            if widgets::button(ui, theme, ButtonKind::Ghost, Some(Icon::Info), "About", true).clicked() {
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
    fn wordmark(&self, ui: &mut egui::Ui, theme: &Theme) {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
            let painter = ui.painter();
            painter.rect_filled(rect, CornerRadius::same(9), theme.palette.accent);
            let inner = rect.shrink(8.0);
            icons::paint(painter, Icon::Package, inner, theme.palette.on_accent, 1.7);
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
                    actions.push(Action::Refresh);
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
                ui.label(
                    RichText::new(format!("vn-launcher {}", vnl_core::VERSION))
                        .font(theme.text.title.clone())
                        .color(theme.palette.text_primary),
                );
                ui.add_space(SPACE_SM);
                ui.label(
                    RichText::new(
                        "A launcher for the vecnode applications. It tracks what is on this \
                         machine, reads each repository's public releases, and starts what you \
                         already have. No account, no token, no sign-in.",
                    )
                    .font(theme.text.small.clone())
                    .color(theme.palette.text_secondary),
                );
                ui.add_space(SPACE_LG);
                ui.horizontal(|ui| {
                    if widgets::button(
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
                    if widgets::button(ui, theme, ButtonKind::Ghost, Some(Icon::Close), "Close", true)
                        .clicked()
                    {
                        self.show_about = false;
                    }
                });
            });

        if !open {
            self.show_about = false;
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
