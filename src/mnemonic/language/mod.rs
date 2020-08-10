use std::fmt;

mod chinese_simplified;
mod chinese_traditional;
mod czech;
mod english;
mod french;
mod italian;
mod japanese;
mod korean;
mod spanish;

/// Language to be used for the mnemonic phrase.
///
/// The English language is always available, other languages are enabled using
/// the compilation features.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Language {
    /// The English language.
    English,
    /// The Simplified Chinese language.
    SimplifiedChinese,
    /// The Traditional Chinese language.
    TraditionalChinese,
    /// The Czech language.
    Czech,
    /// The French language.
    French,
    /// The Italian language.
    Italian,
    /// The Japanese language.
    Japanese,
    /// The Korean language.
    Korean,
    /// The Spanish language.
    Spanish,
}

impl Language {
    /// The list of supported languages.
    /// Language support is managed by compile features.
    pub fn all() -> &'static [Language] {
        &[
            Language::English,
            Language::SimplifiedChinese,
            Language::TraditionalChinese,
            Language::Czech,
            Language::French,
            Language::Italian,
            Language::Japanese,
            Language::Korean,
            Language::Spanish,
        ]
    }

    /// The word list for this language.
    #[inline]
    pub(crate) fn word_list(self) -> &'static [&'static str; 2048] {
        match self {
            Language::English => &english::WORDS,
            Language::SimplifiedChinese => &chinese_simplified::WORDS,
            Language::TraditionalChinese => &chinese_traditional::WORDS,
            Language::Czech => &czech::WORDS,
            Language::French => &french::WORDS,
            Language::Italian => &italian::WORDS,
            Language::Japanese => &japanese::WORDS,
            Language::Korean => &korean::WORDS,
            Language::Spanish => &spanish::WORDS,
        }
    }

    /// Returns true if all words in the list are guaranteed to
    /// only be in this list and not in any other.
    #[inline]
    pub(crate) fn unique_words(self) -> bool {
        match self {
            Language::English => false,
            Language::SimplifiedChinese => false,
            Language::TraditionalChinese => false,
            Language::Czech => true,
            Language::French => false,
            Language::Italian => true,
            Language::Japanese => true,
            Language::Korean => true,
            Language::Spanish => true,
        }
    }

    /// Get words from the word list that start with the given prefix.
    pub fn words_by_prefix(self, prefix: &str) -> &[&'static str] {
        // The words in the word list are ordered lexicographically. This means
        // that we cannot use `binary_search` to find words more efficiently,
        // because the Rust ordering is based on the byte values. However, it
        // does mean that words that share a prefix will follow each other.

        let first = match self.word_list().iter().position(|w| w.starts_with(prefix)) {
            Some(i) => i,
            None => return &[],
        };
        let count = self.word_list()[first..]
            .iter()
            .take_while(|w| w.starts_with(prefix))
            .count();
        &self.word_list()[first..first + count]
    }

    /// Get the index of the word in the word list.
    #[inline]
    pub(crate) fn find_word(self, word: &str) -> Option<usize> {
        self.word_list().iter().position(|w| *w == word)
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ripemd160::Digest;
    use sha2::Sha256;

    use crate::utils::bytes_to_hex_str;

    #[test]
    fn validate_word_list_checksums() {
        //! In this test, we ensure that the word lists are identical.
        //!
        //! They are as follows in the bips repository:
        //! 5c5942792bd8340cb8b27cd592f1015edf56a8c5b26276ee18a482428e7c5726  chinese_simplified.txt
        //! 417b26b3d8500a4ae3d59717d7011952db6fc2fb84b807f3f94ac734e89c1b5f  chinese_traditional.txt
        //! 7e80e161c3e93d9554c2efb78d4e3cebf8fc727e9c52e03b83b94406bdcc95fc  czech.txt
        //! 2f5eed53a4727b4bf8880d8f3f199efc90e58503646d9ff8eff3a2ed3b24dbda  english.txt
        //! ebc3959ab7801a1df6bac4fa7d970652f1df76b683cd2f4003c941c63d517e59  french.txt
        //! d392c49fdb700a24cd1fceb237c1f65dcc128f6b34a8aacb58b59384b5c648c2  italian.txt
        //! 2eed0aef492291e061633d7ad8117f1a2b03eb80a29d0e4e3117ac2528d05ffd  japanese.txt
        //! 9e95f86c167de88f450f0aaf89e87f6624a57f973c67b516e338e8e8b8897f60  korean.txt
        //! 46846a5a0139d1e3cb77293e521c2865f7bcdb82c44e8d0a06a2cd0ecba48c0b  spanish.txt

        let checksums = [
            (
                "5c5942792bd8340cb8b27cd592f1015edf56a8c5b26276ee18a482428e7c5726",
                Language::SimplifiedChinese,
            ),
            (
                "417b26b3d8500a4ae3d59717d7011952db6fc2fb84b807f3f94ac734e89c1b5f",
                Language::TraditionalChinese,
            ),
            (
                "7e80e161c3e93d9554c2efb78d4e3cebf8fc727e9c52e03b83b94406bdcc95fc",
                Language::Czech,
            ),
            (
                "2f5eed53a4727b4bf8880d8f3f199efc90e58503646d9ff8eff3a2ed3b24dbda",
                Language::English,
            ),
            (
                "ebc3959ab7801a1df6bac4fa7d970652f1df76b683cd2f4003c941c63d517e59",
                Language::French,
            ),
            (
                "d392c49fdb700a24cd1fceb237c1f65dcc128f6b34a8aacb58b59384b5c648c2",
                Language::Italian,
            ),
            (
                "2eed0aef492291e061633d7ad8117f1a2b03eb80a29d0e4e3117ac2528d05ffd",
                Language::Japanese,
            ),
            (
                "9e95f86c167de88f450f0aaf89e87f6624a57f973c67b516e338e8e8b8897f60",
                Language::Korean,
            ),
            (
                "46846a5a0139d1e3cb77293e521c2865f7bcdb82c44e8d0a06a2cd0ecba48c0b",
                Language::Spanish,
            ),
        ];

        for &(sum, lang) in &checksums {
            let mut hasher = Sha256::new();
            for (_idx, word) in lang.word_list().iter().enumerate() {
                assert!(::unicode_normalization::is_nfkd(&word));
                hasher.update(format!("{}\n", word));
            }
            assert_eq!(
                bytes_to_hex_str(&hasher.finalize().to_vec()),
                sum,
                "word list for language {} failed checksum check",
                lang,
            );
        }
    }

    #[test]
    fn words_by_prefix() {
        let lang = Language::English;

        let res = lang.words_by_prefix("woo");
        assert_eq!(res, ["wood", "wool"]);

        let res = lang.words_by_prefix("");
        assert_eq!(res.len(), 2048);

        let res = lang.words_by_prefix("woof");
        assert!(res.is_empty());
    }

    #[test]
    fn words_overlaps() {
        use std::collections::HashMap;

        // We keep a map of all words and the languages they occur in.
        // Afterwards, we make sure that no word maps to multiple languages
        // if either of those is guaranteed to have unique words.
        let mut words: HashMap<&str, Vec<Language>> = HashMap::new();
        for lang in Language::all().iter() {
            for word in lang.word_list().iter() {
                let alt = Vec::new();
                words.entry(word).or_insert(alt).push(*lang);
            }
        }

        let mut ok = true;
        for (word, langs) in words.into_iter() {
            if langs.len() == 1 {
                continue;
            }
            if langs.iter().any(|l| l.unique_words()) {
                println!("Word {} is not unique: {:?}", word, langs);
                ok = false;
            }
        }
        assert!(ok);
    }
}
