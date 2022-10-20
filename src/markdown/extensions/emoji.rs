use crate::markdown::extension::TextExtension;

pub struct EmojiConverter;

impl TextExtension for EmojiConverter {
    fn process_text<'a>(&self, text: &'a str) -> String {
        return convert_emojis(text);
    }
}

fn convert_emojis(input: &str) -> String {
    let mut acc = String::with_capacity(input.len());
    let mut parsing_emoji = false;
    let mut emoji_identifier = String::new();

    for c in input.chars() {
        match (c, parsing_emoji) {
            (':', false) => parsing_emoji = true,
            (':', true) => {
                if let Some(emoji) = emojis::lookup(&emoji_identifier) {
                    acc.push_str(emoji.as_str());
                } else {
                    acc.push(':');
                    acc.push_str(&emoji_identifier);
                    acc.push(':');
                }

                parsing_emoji = false;
                emoji_identifier.truncate(0);
            }
            (_, true) => emoji_identifier.push(c),
            (_, false) => acc.push(c),
        }
    }

    if parsing_emoji {
        acc.push(':');
        acc.push_str(&emoji_identifier);
    }

    acc
}
