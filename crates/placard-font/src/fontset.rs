use crate::Font;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    Named(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontStyle {
    Normal,
    Italic,
}

fn normalize_family(family: FontFamily) -> FontFamily {
    match family {
        FontFamily::Named(name) => FontFamily::Named(name.to_ascii_lowercase()),
        other => other,
    }
}

pub struct FontSet {
    fonts: HashMap<(FontFamily, FontWeight, FontStyle), Font>,
}

impl FontSet {
    pub fn new(fallback: Font) -> Self {
        let mut fonts = HashMap::new();
        fonts.insert(
            (FontFamily::SansSerif, FontWeight::Normal, FontStyle::Normal),
            fallback,
        );
        Self { fonts }
    }

    pub fn insert(&mut self, family: FontFamily, weight: FontWeight, style: FontStyle, font: Font) {
        self.fonts
            .insert((normalize_family(family), weight, style), font);
    }

    pub fn get(&self, family: FontFamily, weight: FontWeight, style: FontStyle) -> &Font {
        self.get_family(&family, weight, style).unwrap_or_else(|| {
            self.fonts
                .get(&(FontFamily::SansSerif, FontWeight::Normal, FontStyle::Normal))
                .expect("FontSet::new always inserts a sans-serif/normal/normal fallback")
        })
    }

    pub fn resolve(&self, families: &[FontFamily], weight: FontWeight, style: FontStyle) -> &Font {
        families
            .iter()
            .find_map(|family| self.get_family(family, weight, style))
            .unwrap_or_else(|| self.get(FontFamily::SansSerif, weight, style))
    }

    fn get_family(
        &self,
        family: &FontFamily,
        weight: FontWeight,
        style: FontStyle,
    ) -> Option<&Font> {
        let family = normalize_family(family.clone());
        self.fonts
            .get(&(family.clone(), weight, style))
            .or_else(|| self.fonts.get(&(family.clone(), weight, FontStyle::Normal)))
            .or_else(|| {
                self.fonts
                    .get(&(family, FontWeight::Normal, FontStyle::Normal))
            })
    }

    pub fn has_family(&self, family: &FontFamily) -> bool {
        let family = normalize_family(family.clone());
        self.fonts.keys().any(|(f, _, _)| *f == family)
    }

    pub fn available_families(&self) -> Vec<String> {
        let mut generic = Vec::new();
        let mut named = Vec::new();
        for (family, _, _) in self.fonts.keys() {
            match family {
                FontFamily::SansSerif if !generic.contains(&"sans-serif") => {
                    generic.push("sans-serif")
                }
                FontFamily::Serif if !generic.contains(&"serif") => generic.push("serif"),
                FontFamily::Monospace if !generic.contains(&"monospace") => {
                    generic.push("monospace")
                }
                FontFamily::Named(name) if !named.contains(name) => named.push(name.clone()),
                _ => {}
            }
        }
        generic.sort();
        named.sort();
        generic
            .into_iter()
            .map(str::to_string)
            .chain(named.into_iter().map(|n| format!("\"{n}\"")))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Font;

    fn test_font() -> Font {
        let data = std::fs::read("/usr/share/fonts/liberation/LiberationSans-Regular.ttf")
            .expect("failed to read font");
        Font::parse(&data).expect("failed to parse font")
    }

    #[test]
    fn named_family_lookup_is_case_insensitive() {
        let mut fonts = FontSet::new(test_font());
        fonts.insert(
            FontFamily::Named("arial".into()),
            FontWeight::Normal,
            FontStyle::Normal,
            test_font(),
        );

        assert!(fonts.has_family(&FontFamily::Named("Arial".into())));
        assert!(fonts.has_family(&FontFamily::Named("ARIAL".into())));
        assert!(fonts.has_family(&FontFamily::Named("arial".into())));
        assert!(!fonts.has_family(&FontFamily::Named("Helvetica".into())));
    }

    #[test]
    fn named_family_registered_with_mixed_case_is_still_found_case_insensitively() {
        let mut fonts = FontSet::new(test_font());
        fonts.insert(
            FontFamily::Named("Comic Sans MS".into()),
            FontWeight::Normal,
            FontStyle::Normal,
            test_font(),
        );

        assert!(fonts.has_family(&FontFamily::Named("comic sans ms".into())));
    }

    #[test]
    fn available_families_lists_named_fonts_lowercased_regardless_of_insert_case() {
        let mut fonts = FontSet::new(test_font());
        fonts.insert(
            FontFamily::Named("Comic Sans MS".into()),
            FontWeight::Normal,
            FontStyle::Normal,
            test_font(),
        );

        assert!(
            fonts
                .available_families()
                .contains(&"\"comic sans ms\"".to_string())
        );
    }
}
