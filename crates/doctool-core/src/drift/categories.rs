use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DriftCategory {
    MissingEndpoint,
    OrphanDoc,
    LocaleGap,
    LocaleStale,
    LocaleStructure,
    LocaleOrphan,
    SdkUnmentioned,
    GuideDeadLink,
}

impl DriftCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingEndpoint => "missing_endpoint",
            Self::OrphanDoc => "orphan_doc",
            Self::LocaleGap => "locale_gap",
            Self::LocaleStale => "locale_stale",
            Self::LocaleStructure => "locale_structure",
            Self::LocaleOrphan => "locale_orphan",
            Self::SdkUnmentioned => "sdk_unmentioned",
            Self::GuideDeadLink => "guide_dead_link",
        }
    }

    pub fn suggested_command(self) -> Option<&'static str> {
        match self {
            Self::MissingEndpoint => Some("dt scaffold"),
            Self::LocaleGap => Some("dt sync-i18n --scaffold-missing"),
            Self::LocaleStale => Some("dt sync-i18n lock"),
            Self::LocaleStructure => None,
            Self::LocaleOrphan => None,
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::MissingEndpoint,
            Self::OrphanDoc,
            Self::LocaleGap,
            Self::LocaleStale,
            Self::LocaleStructure,
            Self::LocaleOrphan,
            Self::SdkUnmentioned,
            Self::GuideDeadLink,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_strings_are_stable() {
        assert_eq!(DriftCategory::MissingEndpoint.as_str(), "missing_endpoint");
        assert_eq!(DriftCategory::GuideDeadLink.as_str(), "guide_dead_link");
    }
}
