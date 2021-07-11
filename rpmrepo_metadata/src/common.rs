use std::cmp::Ordering;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::fmt;

use super::MetadataError;

#[derive(Debug, Eq, Default, Clone)]
pub struct EVR {
    pub epoch: String,
    pub version: String, // ver
    pub release: String, // rel
}

impl EVR {
    pub fn new(epoch: &str, version: &str, release: &str) -> EVR {
        EVR {
            epoch: epoch.to_owned(),
            version: version.to_owned(),
            release: release.to_owned(),
        }
    }

    pub fn values(&self) -> (&str, &str, &str) {
        (&self.epoch, &self.version, &self.release)
    }

    pub fn parse_values(evr: &str) -> Result<(&str, &str, &str), MetadataError> {
        let (epoch, vr) = evr.split_once(':').unwrap_or(evr.split_at(0));
        let (version, release) = vr.split_once('-').expect("couldn't find release separator"); // TODO
        Ok((epoch, version, release))
    }

    pub fn parse(evr: &str) -> Result<Self, MetadataError> {
        Ok(EVR::parse_values(evr)?.try_into()?)
    }
}

impl TryFrom<(&str, &str, &str)> for EVR {
    type Error = MetadataError;

    fn try_from(val: (&str, &str, &str)) -> Result<Self, Self::Error> {
        Ok(EVR::new(val.0, val.1, val.2))
    }
}

impl PartialEq for EVR {
    fn eq(&self, other: &Self) -> bool {
        ((self.epoch == other.epoch)
            || (self.epoch == "" && other.epoch == "0")
            || (self.epoch == "0" && other.epoch == ""))
            && self.version == other.version
            && self.release == other.release
    }
}

impl fmt::Display for EVR {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.epoch.is_empty() {
            write!(f, "{}:", self.epoch)?;
        }

        write!(f, "{}-{}", self.version, self.release)
    }
}

impl PartialOrd for EVR {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EVR {
    fn cmp(&self, other: &Self) -> Ordering {
        let epoch_1 = if self.epoch.is_empty() {
            "0"
        } else {
            &self.epoch
        };
        let epoch_2 = if other.epoch.is_empty() {
            "0"
        } else {
            &other.epoch
        };

        let epoch_cmp = compare_version_string(epoch_1, epoch_2);
        if epoch_cmp != Ordering::Equal {
            return epoch_cmp;
        }

        let version_cmp = compare_version_string(&self.version, &other.version);
        if version_cmp != Ordering::Equal {
            return version_cmp;
        }

        compare_version_string(&self.release, &other.release)
    }
}

fn compare_version_string(version1: &str, version2: &str) -> Ordering {
    if version1 == version2 {
        return Ordering::Equal;
    }

    let mut version1_part = version1.clone();
    let mut version2_part = version2.clone();

    let not_alphanumeric_tilde_or_caret =
        |c: char| !c.is_ascii_alphanumeric() && c != '~' && c != '^';

    loop {
        // Strip any leading non-alphanumeric, non-tilde characters
        version1_part = version1_part.trim_start_matches(not_alphanumeric_tilde_or_caret);
        version2_part = version2_part.trim_start_matches(not_alphanumeric_tilde_or_caret);

        // Tilde separator parses as "older" or less
        match (
            version1_part.strip_prefix('~'),
            version2_part.strip_prefix('~'),
        ) {
            (Some(_), None) => return Ordering::Less,
            (None, Some(_)) => return Ordering::Greater,
            (Some(a), Some(b)) => {
                version1_part = a;
                version2_part = b;
                continue;
            }
            _ => (),
        }

        // if two strings are equal but one is longer, the longer one is considered greater
        match (
            version1_part.strip_prefix('^'),
            version2_part.strip_prefix('^'),
        ) {
            (Some(_), None) => match version2_part.is_empty() {
                true => return Ordering::Greater,
                false => return Ordering::Less,
            },
            (None, Some(_)) => match version1.is_empty() {
                true => return Ordering::Less,
                false => return Ordering::Greater,
            },
            (Some(a), Some(b)) => {
                version1_part = a;
                version2_part = b;
                continue;
            }
            _ => (),
        }

        if version1_part.is_empty() || version2_part.is_empty() {
            break;
        }

        fn matching_contiguous<F>(string: &str, pat: F) -> Option<(&str, &str)>
        where
            F: Fn(char) -> bool,
        {
            Some(
                string.split_at(
                    string
                        .find(|c| !pat(c))
                        .or(Some(string.len()))
                        .filter(|&x| x > 0)?,
                ),
            )
        }

        if version1_part.starts_with(|c: char| c.is_ascii_digit()) {
            match (
                matching_contiguous(version1_part, |c| c.is_ascii_digit()),
                matching_contiguous(version2_part, |c| c.is_ascii_digit()),
            ) {
                (Some(a), Some(b)) => {
                    let (prefix1, version1) = a;
                    let (prefix2, version2) = b;
                    version1_part = version1;
                    version2_part = version2;
                    let ordering = prefix1
                        .trim_start_matches('0')
                        .len()
                        .cmp(&prefix2.trim_start_matches('0').len());
                    if ordering != Ordering::Equal {
                        return ordering;
                    }
                    let ordering = prefix1.cmp(&prefix2);
                    if ordering != Ordering::Equal {
                        return ordering;
                    }
                }
                (Some(_), None) => return Ordering::Greater,
                _ => unreachable!(),
            }
        } else {
            match (
                matching_contiguous(version1_part, |c| c.is_ascii_alphabetic()),
                matching_contiguous(version2_part, |c| c.is_ascii_alphabetic()),
            ) {
                (Some(a), Some(b)) => {
                    let (prefix1, version1) = a;
                    let (prefix2, version2) = b;
                    version1_part = version1;
                    version2_part = version2;
                    let ordering = prefix1.cmp(&prefix2);
                    if ordering != Ordering::Equal {
                        return ordering;
                    }
                }
                (Some(_), None) => return Ordering::Less,
                _ => unreachable!(),
            }
        }
    }

    if version1_part.is_empty() && version2_part.is_empty() {
        return Ordering::Equal;
    }

    version1_part.len().cmp(&version2_part.len())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_evr_tostr() {
        let evr = EVR::new("", "1.2.3", "45");
        assert_eq!("1.2.3-45", evr.to_string());

        let evr = EVR::new("0", "1.2.3", "45");
        assert_eq!("0:1.2.3-45", evr.to_string());
    }

    #[test]
    fn test_evr_fromstr() -> Result<(), MetadataError> {
        let evr = EVR::new("", "1.2.3", "45");
        assert_eq!(EVR::parse("1.2.3-45")?, evr);

        let evr = EVR::new("0", "1.2.3", "45");
        assert_eq!(EVR::parse("0:1.2.3-45")?, evr);

        Ok(())
    }

    #[test]
    fn test_evr_ord() -> Result<(), MetadataError> {
        // compare the same EVR without as equal
        let evr1 = EVR::parse("1.2.3-45")?;
        let evr2 = EVR::parse("1.2.3-45")?;
        assert!(evr1 == evr2);

        // compare the same EVR with epoch as equal
        let evr1 = EVR::parse("2:1.2.3-45")?;
        let evr2 = EVR::parse("2:1.2.3-45")?;
        assert!(evr1 == evr2);

        // compare the same EVR with a default epoch as equal
        let evr1 = EVR::parse("1.2.3-45")?;
        let evr2 = EVR::parse("0:1.2.3-45")?;
        assert!(evr1 == evr2);

        // compare EVR with higher epoch and same version / release
        let evr1 = EVR::parse("1.2.3-45")?;
        let evr2 = EVR::parse("1:1.2.3-45")?;
        assert!(evr1 < evr2);

        // compare EVR with higher epoch taken over EVR with higher version
        let evr1 = EVR::parse("4.2.3-45")?;
        let evr2 = EVR::parse("1:1.2.3-45")?;
        assert!(evr1 < evr2);

        // compare versions
        let evr1 = EVR::parse("1.2.3-45")?;
        let evr2 = EVR::parse("1.2.4-45")?;
        assert!(evr1 < evr2);

        // compare versions
        let evr1 = EVR::parse("1.23.3-45")?;
        let evr2 = EVR::parse("1.2.3-45")?;
        assert!(evr1 > evr2);

        // compare versions
        let evr1 = EVR::parse("12.2.3-45")?;
        let evr2 = EVR::parse("1.2.3-45")?;
        assert!(evr1 > evr2);

        // compare versions
        let evr1 = EVR::parse("1.2.3-45")?;
        let evr2 = EVR::parse("1.12.3-45")?;
        assert!(evr1 < evr2);

        // compare versions with tilde parsing as older
        let evr1 = EVR::parse("~1.2.3-45")?;
        let evr2 = EVR::parse("1.2.3-45")?;
        assert!(evr1 < evr2);

        // compare versions with tilde parsing as older
        let evr1 = EVR::parse("~12.2.3-45")?;
        let evr2 = EVR::parse("1.2.3-45")?;
        assert!(evr1 < evr2);

        // compare versions with tilde parsing as older
        let evr1 = EVR::parse("~12.2.3-45")?;
        let evr2 = EVR::parse("~1.2.3-45")?;
        assert!(evr1 > evr2);

        // compare versions with tilde parsing as older
        let evr1 = EVR::parse("~3:12.2.3-45")?;
        let evr2 = EVR::parse("0:1.2.3-45")?;
        assert!(evr1 < evr2);

        // compare release
        let evr1 = EVR::parse("1.2.3-45")?;
        let evr2 = EVR::parse("1.2.3-46")?;
        assert!(evr1 < evr2);

        // compare release
        let evr1 = EVR::parse("1.2.3-3")?;
        let evr2 = EVR::parse("1.2.3-10")?;
        assert!(evr1 < evr2);

        Ok(())
    }

    #[test]
    fn test_compare_version_string() {
        // version comparisons with tilde and caret
        assert_eq!(Ordering::Equal, compare_version_string("1.0^", "1.0^"));
        assert_eq!(Ordering::Greater, compare_version_string("1.0^", "1.0"));
        assert_eq!(Ordering::Less, compare_version_string("1.0", "1.0git1^"));
        assert_eq!(
            Ordering::Less,
            compare_version_string("1.0^git1", "1.0^git2")
        );
        assert_eq!(
            Ordering::Greater,
            compare_version_string("1.01", "1.0^git1")
        );
        assert_eq!(
            Ordering::Equal,
            compare_version_string("1.0^20210501", "1.0^20210501")
        );
        assert_eq!(
            Ordering::Less,
            compare_version_string("1.0^20210501", "1.0.1")
        );
        assert_eq!(
            Ordering::Equal,
            compare_version_string("1.0^20210501^git1", "1.0^20210501^git1")
        );
        assert_eq!(
            Ordering::Greater,
            compare_version_string("1.0^20210502", "1.0^20210501^git1")
        );
        assert_eq!(
            Ordering::Equal,
            compare_version_string("1.0~rc1^git1", "1.0~rc1^git1")
        );
        assert_eq!(
            Ordering::Greater,
            compare_version_string("1.0~rc1^git1", "1.0~rc1")
        );
        assert_eq!(
            Ordering::Equal,
            compare_version_string("1.0^git1~pre", "1.0^git1~pre")
        );
        assert_eq!(
            Ordering::Greater,
            compare_version_string("1.0^git1", "1.0^git1~pre")
        );

        //// non-intuitive behavior
        assert_eq!(Ordering::Less, compare_version_string("1e.fc33", "1.fc33"));
        assert_eq!(
            Ordering::Greater,
            compare_version_string("1g.fc33", "1.fc33")
        );

        //// non-ascii characters compare as the same
        assert_eq!(Ordering::Equal, compare_version_string("1.1.α", "1.1.α"));
        assert_eq!(Ordering::Equal, compare_version_string("1.1.α", "1.1.β"));
        assert_eq!(Ordering::Equal, compare_version_string("1.1.αα", "1.1.α"));
        assert_eq!(Ordering::Equal, compare_version_string("1.1.α", "1.1.ββ"));
    }
}
