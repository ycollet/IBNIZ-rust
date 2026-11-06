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
    Shortest,
    Summerlab,
}

impl Origin {
    fn label(self) -> &'static str {
        match self {
            Self::Original => "Original IBNIZ examples",
            Self::Manual => "IBNIZ language manual",
            Self::Jibniz => "flupe/jibniz gallery",
            Self::Ibnjs => "uones/ibnjs WebGL gallery",
            Self::Shortest => "Shortest IBNIZ programs collection",
            Self::Summerlab => "Summerlab workshop collection",
        }
    }

    fn license(self) -> (&'static str, &'static str) {
        match self {
            Self::Original | Self::Manual => ("Zlib", include_str!("../../LICENSE")),
            Self::Jibniz => ("MIT", include_str!("../../examples/LICENSE-jibniz.txt")),
            Self::Ibnjs => ("WTFPL v2", include_str!("../../examples/LICENSE-ibnjs.txt")),
            Self::Shortest | Self::Summerlab => (
                "Not specified",
                "The bundled source collection does not state an explicit license. Original credits and source references are retained; no additional license is inferred.",
            ),
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
            Origin::Shortest => "https://github.com/ycollet/IBNIZ-rust/blob/f88d5cf5c7232e436ea48cf9a418d261641aece7/examples/shortest_ibniz_programs.txt".into(),
            Origin::Summerlab => "https://github.com/ycollet/IBNIZ-rust/blob/23d77723d1d983bef64e6df803bcc831d10064d9/examples/summerlab-ibniz-latest.txt".into(),
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
    example!(
        "shortest-gradient-sawtooth",
        "Gradient & sawtooth",
        "Extracted from shortest_ibniz_programs.txt, lines 9. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-flasher-slowing-down",
        "flasher slowing down",
        "Extracted from shortest_ibniz_programs.txt, lines 12. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Shortest
    ),
    example!(
        "shortest-glitchy-halfscreen",
        "glitchy halfscreen",
        "Extracted from shortest_ibniz_programs.txt, lines 13. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-colorfade",
        "colorfade",
        "Extracted from shortest_ibniz_programs.txt, lines 14. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-trippy-whining-rotator",
        "trippy & whining rotator",
        "Extracted from shortest_ibniz_programs.txt, lines 18. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Shortest
    ),
    example!(
        "shortest-textured-headache",
        "*-textured headache",
        "Extracted from shortest_ibniz_programs.txt, lines 19. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-stopping-near-negative-star",
        "stopping near negative star",
        "Extracted from shortest_ibniz_programs.txt, lines 21. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Shortest
    ),
    example!(
        "shortest-mcp-variant-1",
        "mcp (variant 1)",
        "Extracted from shortest_ibniz_programs.txt, lines 22. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Shortest
    ),
    example!(
        "shortest-mcp-variant-2",
        "mcp (variant 2)",
        "Extracted from shortest_ibniz_programs.txt, lines 22. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Shortest
    ),
    example!(
        "shortest-mcp-variant-3",
        "mcp (variant 3)",
        "Extracted from shortest_ibniz_programs.txt, lines 22. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Shortest
    ),
    example!(
        "shortest-gradual-art",
        "gradual art",
        "Extracted from shortest_ibniz_programs.txt, lines 23. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Shortest
    ),
    example!(
        "shortest-spotlights-among-textures-variant-1",
        "spotlights among *-textures (variant 1)",
        "Extracted from shortest_ibniz_programs.txt, lines 24. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-spotlights-among-textures-variant-2",
        "spotlights among *-textures (variant 2)",
        "Extracted from shortest_ibniz_programs.txt, lines 24. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-spotlights-among-textures-variant-3",
        "spotlights among *-textures (variant 3)",
        "Extracted from shortest_ibniz_programs.txt, lines 24. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-plasmas",
        "plasmas",
        "Extracted from shortest_ibniz_programs.txt, lines 26. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-moving-glitchsquares",
        "moving glitchsquares",
        "Extracted from shortest_ibniz_programs.txt, lines 27. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-jupiter-storm-variant-3",
        "jupiter storm (variant 3)",
        "Extracted from shortest_ibniz_programs.txt, lines 28. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-jupiter-storm-variant-4",
        "jupiter storm (variant 4)",
        "Extracted from shortest_ibniz_programs.txt, lines 28. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-jupiter-storm-variant-5",
        "jupiter storm (variant 5)",
        "Extracted from shortest_ibniz_programs.txt, lines 28. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-jupiter-storm-variant-6",
        "jupiter storm (variant 6)",
        "Extracted from shortest_ibniz_programs.txt, lines 28. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-jupiter-storm-variant-7",
        "jupiter storm (variant 7)",
        "Extracted from shortest_ibniz_programs.txt, lines 28. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-jupiter-storm-variant-8",
        "jupiter storm (variant 8)",
        "Extracted from shortest_ibniz_programs.txt, lines 28. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-blinking-stripes-variant-1",
        "blinking stripes (variant 1)",
        "Extracted from shortest_ibniz_programs.txt, lines 29. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-blinking-stripes-variant-2",
        "blinking stripes (variant 2)",
        "Extracted from shortest_ibniz_programs.txt, lines 29. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-blinking-stripes-variant-3",
        "blinking stripes (variant 3)",
        "Extracted from shortest_ibniz_programs.txt, lines 29. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-blinking-stripes-variant-4",
        "blinking stripes (variant 4)",
        "Extracted from shortest_ibniz_programs.txt, lines 29. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "shortest-stopping-in-hyperspace-variant-1",
        "stopping in hyperspace (variant 1)",
        "Extracted from shortest_ibniz_programs.txt, lines 30. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Shortest
    ),
    example!(
        "shortest-stopping-in-hyperspace-variant-2",
        "stopping in hyperspace (variant 2)",
        "Extracted from shortest_ibniz_programs.txt, lines 30. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Shortest
    ),
    example!(
        "shortest-stuff-with-sierpinski-triangles-variant-2",
        "stuff with sierpinski triangles (variant 2)",
        "Extracted from shortest_ibniz_programs.txt, lines 31. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Fractals,
        Shortest
    ),
    example!(
        "shortest-stuff-with-sierpinski-triangles-variant-3",
        "stuff with sierpinski triangles (variant 3)",
        "Extracted from shortest_ibniz_programs.txt, lines 31. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Fractals,
        Shortest
    ),
    example!(
        "shortest-stuff-with-sierpinski-triangles-variant-4",
        "stuff with sierpinski triangles (variant 4)",
        "Extracted from shortest_ibniz_programs.txt, lines 31. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Fractals,
        Shortest
    ),
    example!(
        "shortest-stuff-with-sierpinski-triangles-variant-5",
        "stuff with sierpinski triangles (variant 5)",
        "Extracted from shortest_ibniz_programs.txt, lines 31. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Fractals,
        Shortest
    ),
    example!(
        "shortest-stuff-with-sierpinski-triangles-variant-6",
        "stuff with sierpinski triangles (variant 6)",
        "Extracted from shortest_ibniz_programs.txt, lines 31. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Fractals,
        Shortest
    ),
    example!(
        "shortest-stuff-with-sierpinski-triangles-variant-7",
        "stuff with sierpinski triangles (variant 7)",
        "Extracted from shortest_ibniz_programs.txt, lines 31. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Fractals,
        Shortest
    ),
    example!(
        "shortest-munching-epilepsy-variant-1",
        "munching epilepsy (variant 1)",
        "Extracted from shortest_ibniz_programs.txt, lines 32. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Shortest
    ),
    example!(
        "shortest-munching-epilepsy-variant-2",
        "munching epilepsy (variant 2)",
        "Extracted from shortest_ibniz_programs.txt, lines 32. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Shortest
    ),
    example!(
        "shortest-munching-epilepsy-variant-3",
        "munching epilepsy (variant 3)",
        "Extracted from shortest_ibniz_programs.txt, lines 32. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Shortest
    ),
    example!(
        "shortest-noisy-bar-scrolling-down-slowly",
        "noisy bar scrolling down slowly",
        "Extracted from shortest_ibniz_programs.txt, lines 34. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Shortest
    ),
    example!(
        "shortest-random-squeaks",
        "random squeaks",
        "Extracted from shortest_ibniz_programs.txt, lines 35. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Music,
        Shortest
    ),
    example!(
        "shortest-noisy-bar-in-a-textured-night",
        "noisy bar in a *-textured night",
        "Extracted from shortest_ibniz_programs.txt, lines 36. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Shortest
    ),
    example!(
        "summerlab-n-b-miroir",
        "n&b miroir",
        "Extracted from summerlab-ibniz-latest.txt, lines 12. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Textures,
        Summerlab
    ),
    example!(
        "summerlab-binary-jet",
        "binary jet",
        "Extracted from summerlab-ibniz-latest.txt, lines 13. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-gluon",
        "gluon",
        "Extracted from summerlab-ibniz-latest.txt, lines 14. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-new-new-sleep-2",
        "new new sleep 2",
        "Extracted from summerlab-ibniz-latest.txt, lines 15. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-muon",
        "muon",
        "Extracted from summerlab-ibniz-latest.txt, lines 16. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-trumpet-rmx",
        "trumpet rmx",
        "Extracted from summerlab-ibniz-latest.txt, lines 17. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-angelic-hysteron-isogamete",
        "angelic hysteron isogamete",
        "Extracted from summerlab-ibniz-latest.txt, lines 18. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-new-new-sleep",
        "new new sleep",
        "Extracted from summerlab-ibniz-latest.txt, lines 19. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-smmrlb2017",
        "smmrlb2017",
        "Extracted from summerlab-ibniz-latest.txt, lines 20. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-cinderella-berkelium-canoe",
        "cinderella berkelium canoe",
        "Extracted from summerlab-ibniz-latest.txt, lines 25. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-cheeky-anthracite-bacteria",
        "cheeky anthracite bacteria",
        "Extracted from summerlab-ibniz-latest.txt, lines 27. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-snotty-ice-cream-electrode",
        "snotty ice-cream electrode",
        "Extracted from summerlab-ibniz-latest.txt, lines 29. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-sleep",
        "sleep",
        "Extracted from summerlab-ibniz-latest.txt, lines 31. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-lettuce-fumigation-inhaler",
        "lettuce fumigation inhaler",
        "Extracted from summerlab-ibniz-latest.txt, lines 37. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-glucophage-nut-margarine",
        "glucophage nut margarine",
        "Extracted from summerlab-ibniz-latest.txt, lines 39. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-reset",
        "reset",
        "Extracted from summerlab-ibniz-latest.txt, lines 43. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-2012",
        "summerlab 2012",
        "Extracted from summerlab-ibniz-latest.txt, lines 45. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-nutcracker-drag-race",
        "nutcracker drag race",
        "Extracted from summerlab-ibniz-latest.txt, lines 47. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-elongate-generosity-potion",
        "elongate generosity potion",
        "Extracted from summerlab-ibniz-latest.txt, lines 49. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-average-butane-cicada",
        "average butane cicada",
        "Extracted from summerlab-ibniz-latest.txt, lines 51. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-ostrich-junkie-invader",
        "ostrich junkie invader",
        "Extracted from summerlab-ibniz-latest.txt, lines 53. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-aphasia-corrodible-bruise",
        "aphasia corrodible bruise",
        "Extracted from summerlab-ibniz-latest.txt, lines 55. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-chubby-chloroform-ornithology",
        "chubby chloroform ornithology",
        "Extracted from summerlab-ibniz-latest.txt, lines 57. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-premium-spinal-tofu",
        "premium spinal tofu",
        "Extracted from summerlab-ibniz-latest.txt, lines 59. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-black-magic-sunbathing",
        "black magic sunbathing",
        "Extracted from summerlab-ibniz-latest.txt, lines 61. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-chimpanzee-lagoon-bingo",
        "chimpanzee lagoon bingo",
        "Extracted from summerlab-ibniz-latest.txt, lines 63. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("emoc"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-beat-simple",
        "beat simple",
        "Extracted from summerlab-ibniz-latest.txt, lines 68. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("diane"),
        Music,
        Summerlab
    ),
    example!(
        "summerlab-note-simple",
        "note simple",
        "Extracted from summerlab-ibniz-latest.txt, lines 70. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("diane"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-tv-drone",
        "tv drone",
        "Extracted from summerlab-ibniz-latest.txt, lines 76. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("diane"),
        Music,
        Summerlab
    ),
    example!(
        "summerlab-melo",
        "melo?",
        "Extracted from summerlab-ibniz-latest.txt, lines 78. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("diane"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-drone2beat",
        "drone2beat",
        "Extracted from summerlab-ibniz-latest.txt, lines 80. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Music,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-94",
        "Summerlab line 94",
        "Extracted from summerlab-ibniz-latest.txt, lines 94. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-98",
        "Summerlab line 98",
        "Extracted from summerlab-ibniz-latest.txt, lines 98. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-100",
        "Summerlab line 100",
        "Extracted from summerlab-ibniz-latest.txt, lines 100. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-102",
        "Summerlab line 102",
        "Extracted from summerlab-ibniz-latest.txt, lines 102. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-104",
        "Summerlab line 104",
        "Extracted from summerlab-ibniz-latest.txt, lines 104. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-106",
        "Summerlab line 106",
        "Extracted from summerlab-ibniz-latest.txt, lines 106. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-eight-position-sequencer-1",
        "Eight-position sequencer 1",
        "Extracted from summerlab-ibniz-latest.txt, lines 108-116. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-eight-position-sequencer-2",
        "Eight-position sequencer 2",
        "Extracted from summerlab-ibniz-latest.txt, lines 120-128. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-130",
        "Summerlab line 130",
        "Extracted from summerlab-ibniz-latest.txt, lines 130. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-132",
        "Summerlab line 132",
        "Extracted from summerlab-ibniz-latest.txt, lines 132. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-134",
        "Summerlab line 134",
        "Extracted from summerlab-ibniz-latest.txt, lines 134. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-trop-colle",
        "trop colle",
        "Extracted from summerlab-ibniz-latest.txt, lines 137-138. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-143",
        "Summerlab line 143",
        "Extracted from summerlab-ibniz-latest.txt, lines 143. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-drone",
        "drone",
        "Extracted from summerlab-ibniz-latest.txt, lines 149. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Music,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-151",
        "Summerlab line 151",
        "Extracted from summerlab-ibniz-latest.txt, lines 151. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-carre-carre",
        "carre-carre",
        "Extracted from summerlab-ibniz-latest.txt, lines 156. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-158",
        "Summerlab line 158",
        "Extracted from summerlab-ibniz-latest.txt, lines 158. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-162",
        "Summerlab line 162",
        "Extracted from summerlab-ibniz-latest.txt, lines 162. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-176",
        "Summerlab line 176",
        "Extracted from summerlab-ibniz-latest.txt, lines 176. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-179",
        "Summerlab line 179",
        "Extracted from summerlab-ibniz-latest.txt, lines 179. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        None,
        Other,
        Summerlab
    ),
    example!(
        "summerlab-rose-garden",
        "Rose Garden",
        "Extracted from summerlab-ibniz-latest.txt, lines 226. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-color-pattern",
        "Color Pattern",
        "Extracted from summerlab-ibniz-latest.txt, lines 228. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Textures,
        Summerlab
    ),
    example!(
        "summerlab-trumpet-sound",
        "Trumpet sound",
        "Extracted from summerlab-ibniz-latest.txt, lines 229. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Music,
        Summerlab
    ),
    example!(
        "summerlab-mono-tone-patterns",
        "Mono tone Patterns",
        "Extracted from summerlab-ibniz-latest.txt, lines 231. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Textures,
        Summerlab
    ),
    example!(
        "summerlab-shadow-of-mandelbrot",
        "Shadow of Mandelbrot?",
        "Extracted from summerlab-ibniz-latest.txt, lines 232. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Fractals,
        Summerlab
    ),
    example!(
        "summerlab-gray-patterns",
        "Gray Patterns",
        "Extracted from summerlab-ibniz-latest.txt, lines 236. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Textures,
        Summerlab
    ),
    example!(
        "summerlab-numbers-arithmetic-variations",
        "NUMBERS/ARITHMETIC variations",
        "Extracted from summerlab-ibniz-latest.txt, lines 237. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-gray-patterns-a-melody-changes-automatically",
        "Gray Patterns - A melody changes automatically",
        "Extracted from summerlab-ibniz-latest.txt, lines 238. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Music,
        Summerlab
    ),
    example!(
        "summerlab-bar-graph-a-melody-changes-automatically",
        "Bar graph - A melody changes automatically",
        "Extracted from summerlab-ibniz-latest.txt, lines 239. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Music,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-242",
        "Summerlab line 242",
        "Extracted from summerlab-ibniz-latest.txt, lines 242. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-243",
        "Summerlab line 243",
        "Extracted from summerlab-ibniz-latest.txt, lines 243. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-244",
        "Summerlab line 244",
        "Extracted from summerlab-ibniz-latest.txt, lines 244. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-245",
        "Summerlab line 245",
        "Extracted from summerlab-ibniz-latest.txt, lines 245. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-246",
        "Summerlab line 246",
        "Extracted from summerlab-ibniz-latest.txt, lines 246. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-247",
        "Summerlab line 247",
        "Extracted from summerlab-ibniz-latest.txt, lines 247. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-248",
        "Summerlab line 248",
        "Extracted from summerlab-ibniz-latest.txt, lines 248. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-radiation",
        "Radiation",
        "Extracted from summerlab-ibniz-latest.txt, lines 251. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-reverse-rotations",
        "Reverse rotations",
        "Extracted from summerlab-ibniz-latest.txt, lines 252. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-four-rotations",
        "Four rotations",
        "Extracted from summerlab-ibniz-latest.txt, lines 253. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-254",
        "Summerlab line 254",
        "Extracted from summerlab-ibniz-latest.txt, lines 254. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-255",
        "Summerlab line 255",
        "Extracted from summerlab-ibniz-latest.txt, lines 255. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-xor-texture",
        "Xor texture",
        "Extracted from summerlab-ibniz-latest.txt, lines 258. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Textures,
        Summerlab
    ),
    example!(
        "summerlab-xor-texture-plus-sierpinski-triangle",
        "Xor texture plus Sierpinski triangle",
        "Extracted from summerlab-ibniz-latest.txt, lines 259. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Fractals,
        Summerlab
    ),
    example!(
        "summerlab-xor-texture-color-change",
        "Xor texture color change",
        "Extracted from summerlab-ibniz-latest.txt, lines 260. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Textures,
        Summerlab
    ),
    example!(
        "summerlab-pastelcolor-fractal",
        "PastelColor Fractal",
        "Extracted from summerlab-ibniz-latest.txt, lines 261. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Fractals,
        Summerlab
    ),
    example!(
        "summerlab-b-w-fractal",
        "B&W Fractal",
        "Extracted from summerlab-ibniz-latest.txt, lines 262. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Fractals,
        Summerlab
    ),
    example!(
        "summerlab-flowercolor-fractal",
        "FlowerColor Fractal",
        "Extracted from summerlab-ibniz-latest.txt, lines 263. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Fractals,
        Summerlab
    ),
    example!(
        "summerlab-the-fractal-kingdom-rise-and-downfall",
        "The Fractal Kingdom - Rise and Downfall",
        "Extracted from summerlab-ibniz-latest.txt, lines 264. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Fractals,
        Summerlab
    ),
    example!(
        "summerlab-fractal-flower-garden-another-fractal-variations-slow-speed",
        "Fractal Flower Garden, another Fractal variations (slow speed)",
        "Extracted from summerlab-ibniz-latest.txt, lines 266. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Fractals,
        Summerlab
    ),
    example!(
        "summerlab-striped-pattern-in-ms-dos-and-tower-of-chaos-in-javascript",
        "Striped pattern in MS-DOS and Tower of Chaos in Javascript",
        "Extracted from summerlab-ibniz-latest.txt, lines 268. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("T. Tjm"),
        Textures,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-274",
        "Summerlab line 274",
        "Extracted from summerlab-ibniz-latest.txt, lines 274. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-275",
        "Summerlab line 275",
        "Extracted from summerlab-ibniz-latest.txt, lines 275. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-276",
        "Summerlab line 276",
        "Extracted from summerlab-ibniz-latest.txt, lines 276. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-279",
        "Summerlab line 279",
        "Extracted from summerlab-ibniz-latest.txt, lines 279. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-280",
        "Summerlab line 280",
        "Extracted from summerlab-ibniz-latest.txt, lines 280. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-281",
        "Summerlab line 281",
        "Extracted from summerlab-ibniz-latest.txt, lines 281. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-283",
        "Summerlab line 283",
        "Extracted from summerlab-ibniz-latest.txt, lines 283. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-284",
        "Summerlab line 284",
        "Extracted from summerlab-ibniz-latest.txt, lines 284. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-285",
        "Summerlab line 285",
        "Extracted from summerlab-ibniz-latest.txt, lines 285. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-286",
        "Summerlab line 286",
        "Extracted from summerlab-ibniz-latest.txt, lines 286. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-288",
        "Summerlab line 288",
        "Extracted from summerlab-ibniz-latest.txt, lines 288. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-289",
        "Summerlab line 289",
        "Extracted from summerlab-ibniz-latest.txt, lines 289. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-290",
        "Summerlab line 290",
        "Extracted from summerlab-ibniz-latest.txt, lines 290. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-291",
        "Summerlab line 291",
        "Extracted from summerlab-ibniz-latest.txt, lines 291. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-292",
        "Summerlab line 292",
        "Extracted from summerlab-ibniz-latest.txt, lines 292. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-293",
        "Summerlab line 293",
        "Extracted from summerlab-ibniz-latest.txt, lines 293. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-295",
        "Summerlab line 295",
        "Extracted from summerlab-ibniz-latest.txt, lines 295. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-297",
        "Summerlab line 297",
        "Extracted from summerlab-ibniz-latest.txt, lines 297. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-298",
        "Summerlab line 298",
        "Extracted from summerlab-ibniz-latest.txt, lines 298. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-299",
        "Summerlab line 299",
        "Extracted from summerlab-ibniz-latest.txt, lines 299. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-300",
        "Summerlab line 300",
        "Extracted from summerlab-ibniz-latest.txt, lines 300. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-301",
        "Summerlab line 301",
        "Extracted from summerlab-ibniz-latest.txt, lines 301. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("Glisic"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-373",
        "Summerlab line 373",
        "Extracted from summerlab-ibniz-latest.txt, lines 373. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("viznut"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-374",
        "Summerlab line 374",
        "Extracted from summerlab-ibniz-latest.txt, lines 374. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("viznut"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-376",
        "Summerlab line 376",
        "Extracted from summerlab-ibniz-latest.txt, lines 376. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("viznut"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-422",
        "Summerlab line 422",
        "Extracted from summerlab-ibniz-latest.txt, lines 422. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("geb"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-434",
        "Summerlab line 434",
        "Extracted from summerlab-ibniz-latest.txt, lines 434. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("geb"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-439",
        "Summerlab line 439",
        "Extracted from summerlab-ibniz-latest.txt, lines 439. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("geb"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-442",
        "Summerlab line 442",
        "Extracted from summerlab-ibniz-latest.txt, lines 442. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("geb"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-445",
        "Summerlab line 445",
        "Extracted from summerlab-ibniz-latest.txt, lines 445. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("geb"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-446",
        "Summerlab line 446",
        "Extracted from summerlab-ibniz-latest.txt, lines 446. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("geb"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-449",
        "Summerlab line 449",
        "Extracted from summerlab-ibniz-latest.txt, lines 449. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("geb"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-450",
        "Summerlab line 450",
        "Extracted from summerlab-ibniz-latest.txt, lines 450. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("geb"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-452",
        "Summerlab line 452",
        "Extracted from summerlab-ibniz-latest.txt, lines 452. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("geb"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-455",
        "Summerlab line 455",
        "Extracted from summerlab-ibniz-latest.txt, lines 455. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("geb"),
        Other,
        Summerlab
    ),
    example!(
        "summerlab-summerlab-line-460",
        "Summerlab line 460",
        "Extracted from summerlab-ibniz-latest.txt, lines 460. May flash rapidly or produce loud audio; some programs need a higher instruction budget.",
        Some("geb"),
        Other,
        Summerlab
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

    pub fn show(&mut self, ctx: &egui::Context, managed: bool) -> Option<&'static Example> {
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
                    if ui.button(if managed { "Import example" } else { "Load example" }).clicked() {
                        load = Some(example);
                    }
                    ui.small(if managed {
                        "Importing resets and pauses playback. Save stays in the NSM project."
                    } else {
                        "Loading resets and pauses playback. Save writes a local copy."
                    });
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
    use ibniz::{Program, Vm};
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
                Origin::Shortest => "f88d5cf5c7232e436ea48cf9a418d261641aece7",
                Origin::Summerlab => "23d77723d1d983bef64e6df803bcc831d10064d9",
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
        assert_eq!(browser.visible().len(), EXAMPLES.len());
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
    fn extracted_collections_preserve_programs_and_record_exclusions() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
        let index = include_str!("../../examples/collection-index.tsv");
        let mut extracted = BTreeSet::new();
        for row in index.lines().skip(1) {
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields.len(), 6, "{row}");
            let [collection, location, filename, _, status, reason] = fields.as_slice() else {
                unreachable!()
            };
            if *status == "skipped" {
                assert!(filename.is_empty());
                assert!(!reason.is_empty());
                continue;
            }
            assert!(matches!(*status, "included" | "reused" | "duplicate"));
            let example = EXAMPLES
                .iter()
                .find(|example| example.filename == *filename)
                .unwrap();
            let original = fs::read_to_string(directory.join(collection)).unwrap();
            let bounds: Vec<usize> = location
                .split('-')
                .map(|value| value.parse().unwrap())
                .collect();
            let lines: Vec<_> = original.lines().collect();
            let selected = &lines[bounds[0] - 1..*bounds.last().unwrap()];
            let source = if selected[0].starts_with("- ") {
                if let Some((_, variants)) = selected[0].split_once(':') {
                    variants
                        .split_whitespace()
                        .find(|source| {
                            Program::compile(source).unwrap().instructions()
                                == Program::compile(example.source).unwrap().instructions()
                        })
                        .unwrap()
                        .to_owned()
                } else {
                    assert!(selected[0].contains("gradient & sawtooth"));
                    String::new()
                }
            } else {
                selected
                    .iter()
                    .map(|line| {
                        let line = line.trim();
                        let line = line.strip_prefix('\\').unwrap_or(line);
                        line.split('\\').next().unwrap().trim()
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let expected = Program::compile(&source).unwrap();
            let actual = Program::compile(example.source).unwrap();
            assert_eq!(actual.instructions(), expected.instructions(), "{filename}");
            assert_eq!(actual.data_bits(), expected.data_bits(), "{filename}");
            assert_eq!(
                Vm::new(actual).memory(),
                Vm::new(expected).memory(),
                "{filename}"
            );
            if *status == "included" {
                assert!(extracted.insert(*filename));
                let mut vm = Vm::new(Program::compile(example.source).unwrap());
                let frame = vm.render_frame(4096);
                assert!(frame.instructions <= 4096, "{filename}");
            }
        }
        let files: BTreeSet<_> = EXAMPLES
            .iter()
            .filter(|example| matches!(example.origin, Origin::Shortest | Origin::Summerlab))
            .map(|example| example.filename)
            .collect();
        assert_eq!(extracted, files);
    }

    #[test]
    fn shortest_alternatives_and_empty_program_are_all_available() {
        let index = include_str!("../../examples/collection-index.tsv");
        for line in include_str!("../../examples/shortest_ibniz_programs.txt").lines() {
            let Some((_, variants)) = line
                .strip_prefix("- ")
                .and_then(|line| line.split_once(':'))
            else {
                continue;
            };
            for source in variants.split_whitespace() {
                let program = Program::compile(source).unwrap();
                assert!(
                    index
                        .lines()
                        .skip(1)
                        .filter_map(|row| {
                            let fields: Vec<_> = row.split('\t').collect();
                            (fields[0] == "shortest_ibniz_programs.txt").then_some(fields[2])
                        })
                        .any(|filename| {
                            let example = EXAMPLES
                                .iter()
                                .find(|example| example.filename == filename)
                                .unwrap();
                            Program::compile(example.source).unwrap().instructions()
                                == program.instructions()
                        }),
                    "{source}"
                );
            }
        }
        let empty = EXAMPLES
            .iter()
            .find(|example| example.filename == "shortest-gradient-sawtooth.ib")
            .unwrap();
        assert_eq!(
            Program::compile(empty.source).unwrap().instructions(),
            &[ibniz::Instruction::End]
        );
        let mut browser = Browser {
            search: "summerlab muon".into(),
            ..Default::default()
        };
        assert_eq!(browser.visible().len(), 1);
        assert_eq!(EXAMPLES[browser.visible()[0]].filename, "summerlab-muon.ib");
        browser.search = "shortest mcp".into();
        assert_eq!(browser.visible().len(), 3);
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
            assert!(browser.show(ctx, false).is_none());
        });
        assert_eq!(EXAMPLES[browser.selected.unwrap()].filename, "plasma.ib");
        browser.search = "no such program exists".into();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            assert!(browser.show(ctx, false).is_none());
        });
        assert!(browser.selected.is_none());
    }
}
