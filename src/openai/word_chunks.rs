use std::num::NonZeroUsize;

pub struct WordChunks<'a> {
    content: &'a str,
    chunk_length: NonZeroUsize,
}

impl<'a> WordChunks<'a> {
    #[inline]
    pub fn from_str(content: &'a str, chunk_length: usize) -> Self {
        Self {
            content: content.trim(),
            chunk_length: NonZeroUsize::new(chunk_length).expect("chunk_length must be non-zero"),
        }
    }
}

impl<'a> Iterator for WordChunks<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.content.is_empty() {
            return None;
        }

        if self.content.len() <= self.chunk_length.get() {
            let chunk = self.content.trim();
            self.content = "";
            return Some(chunk);
        }

        let mut boundary = self.chunk_length.get();
        while !self.content.is_char_boundary(boundary) {
            boundary -= 1;
        }

        let mut end = boundary;
        while !self.content.is_char_boundary(end)
            || !self.content[end..].starts_with(char::is_whitespace)
        {
            end -= 1;
            if end == 0 {
                end = boundary;
                break;
            }
        }

        let chunk = &self.content[..end].trim_end();
        self.content = self.content[end..].trim_start();
        Some(chunk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_chunks() {
        let chunks: Vec<_> = WordChunks::from_str("Hello, world! This is a test.", 16).collect();
        assert_eq!(chunks, vec!["Hello, world!", "This is a test."]);

        let chunks: Vec<_> =
            WordChunks::from_str("Hello, world! This is a testingwithlongword.", 16).collect();
        assert_eq!(
            chunks,
            vec!["Hello, world!", "This is a", "testingwithlongw", "ord."]
        );

        let chunks: Vec<_> = WordChunks::from_str("Hello, world!", 16).collect();
        assert_eq!(chunks, vec!["Hello, world!"]);

        let chunks: Vec<_> = WordChunks::from_str("Hello, world! ", 16).collect();
        assert_eq!(chunks, vec!["Hello, world!"]);

        let chunks: Vec<_> = WordChunks::from_str("Hello, world!", 8).collect();
        assert_eq!(chunks, vec!["Hello,", "world!"]);

        let chunks: Vec<_> = WordChunks::from_str("Hello, world!", 7).collect();
        assert_eq!(chunks, vec!["Hello,", "world!"]);

        let chunks: Vec<_> = WordChunks::from_str("Hello, world!", 6).collect();
        assert_eq!(chunks, vec!["Hello,", "world!"]);

        let chunks: Vec<_> = WordChunks::from_str("Hello, world!", 5).collect();
        assert_eq!(chunks, vec!["Hello", ",", "world", "!"]);

        let chunks: Vec<_> = WordChunks::from_str("Hello, world!", 2).collect();
        assert_eq!(chunks, vec!["He", "ll", "o,", "wo", "rl", "d!"]);

        let chunks: Vec<_> = WordChunks::from_str("Hello, world!", 1).collect();
        assert_eq!(
            chunks,
            vec!["H", "e", "l", "l", "o", ",", "w", "o", "r", "l", "d", "!"]
        );
    }
}
