use eframe::egui::{self, Color32};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Fractals,
    Textures,
    Music,
    Demos,
    Other,
}

impl Category {
    const ALL: [Self; 5] = [
        Self::Fractals,
        Self::Textures,
        Self::Music,
        Self::Demos,
        Self::Other,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Fractals => "Fractals",
            Self::Textures => "Textures",
            Self::Music => "Music",
            Self::Demos => "Demos",
            Self::Other => "Other",
        }
    }
}

#[derive(Clone, Copy)]
enum Origin {
    Original,
    Manual,
    Jibniz,
    Ibnjs,
}

impl Origin {
    fn label(self) -> &'static str {
        match self {
            Self::Original => "Original IBNIZ examples",
            Self::Manual => "IBNIZ language manual",
            Self::Jibniz => "flupe/jibniz gallery",
            Self::Ibnjs => "uones/ibnjs WebGL gallery",
        }
    }

    fn license(self) -> (&'static str, &'static str) {
        match self {
            Self::Original | Self::Manual => ("Zlib", include_str!("../../LICENSE")),
            Self::Jibniz => ("MIT", include_str!("../../examples/LICENSE-jibniz.txt")),
            Self::Ibnjs => ("WTFPL v2", include_str!("../../examples/LICENSE-ibnjs.txt")),
        }
    }
}

pub struct Example {
    pub filename: &'static str,
    pub title: &'static str,
    pub source: &'static str,
    description: &'static str,
    contributor: Option<&'static str>,
    category: Category,
    origin: Origin,
}

impl Example {
    fn source_url(&self) -> String {
        match self.origin {
            Origin::Original => format!(
                "https://github.com/viznut/IBNIZ/blob/3ba06ae5f2cf5765c0e06874a093f767054b6359/src/examples/{}",
                self.filename,
            ),
            Origin::Manual => "https://github.com/viznut/IBNIZ/blob/3ba06ae5f2cf5765c0e06874a093f767054b6359/src/ibniz.txt".into(),
            Origin::Jibniz => "https://github.com/flupe/jibniz/blob/009c312c06938cf30d3b1b50572c51989f0f80ee/index.html".into(),
            Origin::Ibnjs => "https://github.com/uones/ibnjs/blob/03314c9bdb5379527dd0899f168c5e32bd339d41/index.html".into(),
        }
    }

    fn matches(&self, query: &str, category: Option<Category>) -> bool {
        if category.is_some_and(|category| category != self.category) {
            return false;
        }
        let text = format!(
            "{} {} {} {} {} {}",
            self.filename,
            self.title,
            self.description,
            self.contributor.unwrap_or(""),
            self.category.label(),
            self.origin.label(),
        )
        .to_lowercase();
        query.split_whitespace().all(|word| text.contains(word))
    }
}

macro_rules! example {
    ($file:literal, $title:literal, $description:literal, $contributor:expr, $category:ident, $origin:ident) => {
        Example {
            filename: concat!($file, ".ib"),
            title: $title,
            source: include_str!(concat!("../../examples/", $file, ".ib")),
            description: $description,
            contributor: $contributor,
            category: Category::$category,
            origin: Origin::$origin,
        }
    };
}

pub static EXAMPLES: &[Example] = &[
    example!(
        "42-melody",
        "42 melody",
        "The manual's compact 42 melody audio program.",
        None,
        Music,
        Manual
    ),
    example!(
        "animated-mandelbrot-set",
        "Animated Mandelbrot set",
        "Animated fractal from the jibniz gallery. May need a higher instruction budget.",
        None,
        Fractals,
        Jibniz
    ),
    example!(
        "bitmapzoomer",
        "Bitmap zoomer",
        "Bitmap/data-segment zoomer from the original examples.",
        None,
        Textures,
        Original
    ),
    example!(
        "bubsy",
        "Looks funny after a while",
        "Gallery program originally described as \"Looks funny after a while\".",
        Some("bubsy"),
        Other,
        Ibnjs
    ),
    example!(
        "christmas-demo",
        "Christmas demo",
        "FreeFull's Christmas demo from the IBNIZ-js gallery.",
        Some("FreeFull"),
        Demos,
        Ibnjs
    ),
    example!(
        "flasher",
        "Flasher",
        "The manual's flasher example. Watch out for rapid flashing.",
        None,
        Other,
        Manual
    ),
    example!(
        "four-char-demo",
        "Four-character demo",
        "Four-character gallery demo by asiekierka.",
        Some("asiekierka"),
        Demos,
        Ibnjs
    ),
    example!(
        "glass",
        "Glass",
        "Gallery example titled Glass, by asiekierka.",
        Some("asiekierka"),
        Textures,
        Ibnjs
    ),
    example!(
        "ibnizdemo",
        "IBNIZ demo",
        "Viznut's original 122-character audiovisual demo.",
        Some("Ville-Matias Heikkila (viznut)"),
        Demos,
        Original
    ),
    example!(
        "interactive-julia-sets",
        "Interactive Julia sets",
        "Move the mouse over the video to explore Julia sets. May need a higher instruction budget.",
        None,
        Fractals,
        Jibniz
    ),
    example!(
        "julia",
        "Julia morpher",
        "Original Julia morpher; real_het is credited in the language manual.",
        Some("real_het"),
        Fractals,
        Original
    ),
    example!(
        "julia-sets-morpher",
        "Julia sets morpher",
        "An alternative Julia morpher from the jibniz gallery.",
        None,
        Fractals,
        Jibniz
    ),
    example!(
        "jupiter-storm",
        "Jupiter storm",
        "The manual's Jupiter storm texture.",
        None,
        Textures,
        Manual
    ),
    example!(
        "jupiter-storm-bw",
        "Jupiter storm, black and white",
        "Black-and-white variant of the Jupiter storm texture.",
        None,
        Textures,
        Manual
    ),
    example!(
        "mandelbrot",
        "Mandelbrot zoomer",
        "Original Mandelbrot zoomer. May need a higher instruction budget.",
        None,
        Fractals,
        Original
    ),
    example!(
        "merging-thing",
        "Merging thing???",
        "Gallery program with its original title, Merging thing???.",
        None,
        Other,
        Jibniz
    ),
    example!(
        "mindrape01",
        "mindrape01",
        "Named gallery example by asiekierka.",
        Some("asiekierka"),
        Textures,
        Jibniz
    ),
    example!(
        "mul-texture-zoomer",
        "Multiplication texture zoomer",
        "Two-character multiplication texture zoomer from the manual.",
        None,
        Textures,
        Manual
    ),
    example!(
        "munching-squares",
        "Munching squares",
        "Munching squares with a Sierpinski harmony.",
        None,
        Textures,
        Manual
    ),
    example!(
        "munching-zoomer",
        "Munching squares zoomer",
        "Zooming variant of the manual's munching squares texture.",
        None,
        Textures,
        Manual
    ),
    example!(
        "music-from-the-video",
        "Music from the video",
        "Music from the original IBNIZ demonstration video.",
        None,
        Music,
        Manual
    ),
    example!(
        "oneliners",
        "One-liner collection",
        "Commented collection: uncomment one program in the editor before running it.",
        None,
        Other,
        Original
    ),
    example!(
        "opening-gate",
        "Opening gate",
        "FreeFull's Opening gate example from the manual.",
        Some("FreeFull"),
        Textures,
        Manual
    ),
    example!(
        "perspective-mapper",
        "Perspective mapper",
        "Perspective-mapping example from the language manual.",
        None,
        Textures,
        Manual
    ),
    example!(
        "plasma",
        "Plasma",
        "The language manual's plasma texture.",
        None,
        Textures,
        Manual
    ),
    example!(
        "polyrhythmic-flasher",
        "Polyrhythmic flasher",
        "Polyrhythmic flasher slowing down, from the manual.",
        None,
        Other,
        Manual
    ),
    example!(
        "rose",
        "Rose",
        "FreeFull's Rose example from the jibniz gallery.",
        Some("FreeFull"),
        Textures,
        Jibniz
    ),
    example!(
        "rotozoomer",
        "Rotozoomer (AND)",
        "The manual's AND variant of the rotozoomer.",
        None,
        Textures,
        Manual
    ),
    example!(
        "rotozoomer-ibnjs",
        "Rotozoomer (XOR)",
        "The IBNIZ-js gallery's XOR variant of the rotozoomer.",
        None,
        Textures,
        Ibnjs
    ),
    example!(
        "sierpinski",
        "Sierpinski",
        "Originally titled \"Sierpinski epilepsy\" in the manual. Contains rapid flashing.",
        None,
        Fractals,
        Manual
    ),
    example!(
        "sliding-squarewave",
        "Sliding-down squarewave",
        "Sliding-down squarewave audio example from the manual.",
        None,
        Music,
        Manual
    ),
    example!(
        "spinny",
        "Spinny",
        "FreeFull's Spinny example from the manual.",
        Some("FreeFull"),
        Textures,
        Manual
    ),
    example!(
        "texture-tunnel",
        "Texture tunnel",
        "Texture tunnel from the manual; distinct from the gallery's XOR tunnel.",
        None,
        Textures,
        Manual
    ),
    example!(
        "tv-noise",
        "TV noise",
        "The manual's TV noise example, without sound.",
        None,
        Textures,
        Manual
    ),
    example!(
        "xor-texture",
        "XOR texture",
        "XOR texture from the jibniz gallery.",
        None,
        Textures,
        Jibniz
    ),
    example!(
        "xor-texture-zoomer",
        "XOR texture zoomer",
        "Zooming XOR texture from the language manual.",
        None,
        Textures,
        Manual
    ),
    example!(
        "xor-tunnel",
        "XOR tunnel",
        "Gallery XOR tunnel with a different rotation from the manual's tunnel.",
        None,
        Textures,
        Jibniz
    ),
    example!(
        "xorwiggle",
        "Xorwiggle",
        "FreeFull's Xorwiggle example from the IBNIZ-js gallery.",
        Some("FreeFull"),
        Textures,
        Ibnjs
    ),
];

#[derive(Default)]
pub struct Browser {
    pub open: bool,
    search: String,
    category: Option<Category>,
    selected: Option<usize>,
}

impl Browser {
    fn visible(&self) -> Vec<usize> {
        let query = self.search.to_lowercase();
        EXAMPLES
            .iter()
            .enumerate()
            .filter(|(_, example)| example.matches(&query, self.category))
            .map(|(index, _)| index)
            .collect()
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<&'static Example> {
        if !self.open {
            return None;
        }
        let mut open = self.open;
        let mut load = None;
        egui::Window::new("Bundled examples")
            .open(&mut open)
            .default_width(760.0)
            .vscroll(true)
            .show(ctx, |ui| {
                ui.colored_label(
                    Color32::YELLOW,
                    "Some examples flash rapidly and produce loud noise. Keep volume low.",
                );
                ui.horizontal(|ui| {
                    ui.label("Search");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Title, filename, description, contributor or source")
                            .desired_width(f32::INFINITY),
                    );
                });
                egui::ComboBox::from_id_salt("example-category")
                    .selected_text(self.category.map_or("All groups", Category::label))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.category, None, "All groups");
                        for category in Category::ALL {
                            ui.selectable_value(
                                &mut self.category,
                                Some(category),
                                category.label(),
                            );
                        }
                    });
                let visible = self.visible();
                if !self.selected.is_some_and(|index| visible.contains(&index)) {
                    self.selected = visible.first().copied();
                }
                ui.small(format!("{} / {} examples", visible.len(), EXAMPLES.len()));
                if visible.is_empty() {
                    ui.label("No examples match. Clear the search or choose another group.");
                    return;
                }
                ui.columns(2, |columns| {
                    egui::ScrollArea::vertical()
                        .id_salt("example-list")
                        .max_height(320.0)
                        .show(&mut columns[0], |ui| {
                            for index in visible {
                                ui.selectable_value(
                                    &mut self.selected,
                                    Some(index),
                                    EXAMPLES[index].title,
                                )
                                .on_hover_text(EXAMPLES[index].filename);
                            }
                        });
                    let ui = &mut columns[1];
                    let example = &EXAMPLES[self.selected.expect("visible example selected")];
                    ui.heading(example.title);
                    ui.small(format!("{} | {}", example.category.label(), example.filename));
                    ui.label(example.description);
                    ui.label(format!(
                        "Contributor: {}",
                        example.contributor.unwrap_or("not individually credited by this source"),
                    ));
                    ui.hyperlink_to(example.origin.label(), example.source_url());
                    let (license, text) = example.origin.license();
                    ui.collapsing(format!("Collection license: {license}"), |ui| {
                        ui.small("Collection licenses do not imply that maintainers authored every program.");
                        egui::ScrollArea::vertical()
                            .id_salt("example-license")
                            .max_height(120.0)
                            .show(ui, |ui| { ui.label(text); });
                    });
                    if ui.button("Load example").clicked() {
                        load = Some(example);
                    }
                    ui.small("Loading resets and pauses playback. Save writes a local copy.");
                });
                if let Some(index) = self.selected {
                    ui.collapsing("Source preview", |ui| {
                        egui::ScrollArea::both()
                            .id_salt("example-source")
                            .max_height(140.0)
                            .show(ui, |ui| {
                                let mut source = EXAMPLES[index].source;
                                ui.add(egui::TextEdit::multiline(&mut source).code_editor());
                            });
                    });
                }
            });
        self.open = open;
        load
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ibniz::Program;
    use std::{collections::BTreeSet, fs, path::Path};

    #[test]
    fn catalog_embeds_every_bundled_program_with_metadata_and_pinned_sources() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
        let files: BTreeSet<_> = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "ib"))
            .map(|path| path.file_name().unwrap().to_str().unwrap().to_owned())
            .collect();
        let embedded: BTreeSet<_> = EXAMPLES
            .iter()
            .map(|example| example.filename.to_owned())
            .collect();
        assert_eq!(embedded, files);
        assert_eq!(embedded.len(), EXAMPLES.len());
        assert_eq!(EXAMPLES.len(), 38);
        for example in EXAMPLES {
            assert_eq!(
                example.source,
                fs::read_to_string(directory.join(example.filename)).unwrap(),
            );
            Program::compile(example.source)
                .unwrap_or_else(|error| panic!("{} does not compile: {error}", example.filename));
            assert!(!example.title.is_empty());
            assert!(!example.description.is_empty());
            let url = example.source_url();
            assert!(url.starts_with("https://github.com/"));
            let revision = match example.origin {
                Origin::Original | Origin::Manual => "3ba06ae5f2cf5765c0e06874a093f767054b6359",
                Origin::Jibniz => "009c312c06938cf30d3b1b50572c51989f0f80ee",
                Origin::Ibnjs => "03314c9bdb5379527dd0899f168c5e32bd339d41",
            };
            assert!(url.contains(revision));
            assert!(!example.origin.license().1.is_empty());
        }
        for category in Category::ALL {
            assert!(EXAMPLES.iter().any(|example| example.category == category));
        }
    }

    #[test]
    fn search_matches_metadata_case_insensitively_and_combines_with_category() {
        let mut browser = Browser::default();
        assert_eq!(browser.visible().len(), 38);
        browser.search = "  FrEeFuLl  MANUAL ".into();
        let visible = browser.visible();
        assert_eq!(visible.len(), 2);
        assert!(visible.iter().all(|&index| {
            EXAMPLES[index].contributor == Some("FreeFull")
                && matches!(EXAMPLES[index].origin, Origin::Manual)
        }));
        browser.category = Some(Category::Music);
        assert!(browser.visible().is_empty());
        browser.search = "42-melody.ib".into();
        assert_eq!(EXAMPLES[browser.visible()[0]].filename, "42-melody.ib");
        browser.search = "uncomment".into();
        browser.category = None;
        assert_eq!(EXAMPLES[browser.visible()[0]].filename, "oneliners.ib");
    }

    #[test]
    fn filtering_reselects_visible_examples_and_clears_hidden_selection() {
        let ctx = egui::Context::default();
        let mut browser = Browser {
            open: true,
            search: "plasma".into(),
            ..Default::default()
        };
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            assert!(browser.show(ctx).is_none());
        });
        assert_eq!(EXAMPLES[browser.selected.unwrap()].filename, "plasma.ib");
        browser.search = "no such program exists".into();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            assert!(browser.show(ctx).is_none());
        });
        assert!(browser.selected.is_none());
    }
}
