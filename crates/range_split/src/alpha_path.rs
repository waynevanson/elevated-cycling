use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlphaPathSegment(pub String);

impl AsRef<Path> for AlphaPathSegment {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl AlphaPathSegment {
    pub fn from_factor(factor: usize) -> Self {
        AlphaPathSegment(vec!['a'; factor].into_iter().collect())
    }
}

// aa -> ab
// az -> ba
// zz -> zza
//
// from end
// if a, we don't have to add anything and increment
// if z, may have to add something if they're all z.

impl AlphaPathSegment {
    pub fn increment_mut(&mut self) {
        // true when we need to iterate and update;
        // false when we need to check for all z's and don't need to update.
        let mut update = true;

        // true when we think they're all z's and we need to append
        // an a to the end.
        let mut all_zs = None;

        let bytes_mut = unsafe { self.0.as_bytes_mut() };
        let bytes = bytes_mut.iter_mut().rev();

        // TODO: optimise for az->ba  rather than zz->zza
        for byte in bytes {
            if !matches!(all_zs, Some(true)) && !update {
                break;
            }

            match byte {
                b'z' => {
                    if all_zs.is_none() {
                        all_zs = Some(true);
                    };

                    if update {
                        *byte = b'a';
                    };
                }
                b'a'..b'z' => {
                    // not all z's
                    all_zs = Some(false);

                    // don't need to update the next because
                    // we consumed it here.
                    if update {
                        *byte += 1;
                        update = false;
                    }
                }
                _ => panic!("should only contain [a-z] all lowercase"),
            }
        }

        if let Some(true) = all_zs {
            for byte in bytes_mut.iter_mut() {
                *byte = b'z';
            }

            self.0.push('a');
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_alpha_segment(input: &str, expected: &str) {
        let mut segment = AlphaPathSegment(input.to_string());
        segment.increment_mut();
        assert_eq!(AlphaPathSegment(expected.to_string()), segment);
    }

    #[test]
    fn aa_ab() {
        let input = "aa";
        let expected = "ab";

        test_alpha_segment(input, expected);
    }

    #[test]
    fn az_ba() {
        let input = "az";
        let expected = "ba";

        test_alpha_segment(input, expected);
    }

    #[test]
    fn zz_zza() {
        let input = "zz";
        let expected = "zza";

        test_alpha_segment(input, expected);
    }
}
