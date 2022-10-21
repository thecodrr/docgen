use std::io::Write;

use crate::markdown::extension::TextExtension;

pub struct EmojiConverter;

impl TextExtension for EmojiConverter {
    fn process_text<'a>(&self, text: &'a str) -> String {
        let mut v = Vec::new();
        convert_emojis(text, &mut v).unwrap();
        return String::from_utf8(v).unwrap();
    }
}

fn convert_emojis(mut input: &str, mut output: impl Write) -> std::io::Result<()> {
    while let Some((i, m, n, j)) = input
        .find(':')
        .map(|i| (i, i + 1))
        .and_then(|(i, m)| input[m..].find(':').map(|x| (i, m, m + x, m + x + 1)))
    {
        match emojis::get_by_shortcode(&input[m..n]) {
            Some(emoji) => {
                // Output everything preceding, except the first colon.
                output.write_all(input[..i].as_bytes())?;
                // Output the emoji.
                output.write_all(emoji.as_bytes())?;
                // Update the string to past the last colon.
                input = &input[j..];
            }
            None => {
                // Output everything preceding but not including the colon.
                output.write_all(input[..n].as_bytes())?;
                // Update the string to start with the last colon.
                input = &input[n..];
            }
        }
    }
    output.write_all(input.as_bytes())
}
