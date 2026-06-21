use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DriftCategory {
    MissingEndpoint,
    OrphanDoc,
    LocaleGap,
    SdkUnmentioned,
    GuideDeadLink,
}

impl DriftCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingEndpoint => "missing_endpoint",
            Self::OrphanDoc => "orphan_doc",
            Self::LocaleGap => "locale_gap",
            Self::SdkUnmentioned => "sdk_unmentioned",
            Self::GuideDeadLink => "guide_dead_link",
        }
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
