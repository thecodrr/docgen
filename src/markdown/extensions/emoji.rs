use pulldown_cmark::CowStr;
use regex::{Captures, Regex};

use crate::markdown::extension::TextExtension;

pub struct EmojiConverter;

lazy_static! {
    static ref EMOJI_REGEX: Regex = Regex::new(r":([a-zA-Z0-9+\-_]+):").unwrap();
}

impl TextExtension for EmojiConverter {
    fn process_text(&self, text: &CowStr) -> CowStr {
        CowStr::from(
            EMOJI_REGEX
                .replace_all(&text, |c: &Captures| {
                    c.get(1)
                        .and_then(|m| emojis::get_by_shortcode(m.as_str()))
                        .map_or_else(|| c.get(0).unwrap().as_str().to_string(), |m| m.to_string())
                })
                .to_string(),
        )
    }
}
