use crate::Version;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RequirementError {
    #[error("Empty requirement string")]
    Empty,
    #[error("Invalid requirement operator: {operator}")]
    InvalidOperator { operator: String },
    #[error("Invalid version in requirement: {version}")]
    InvalidVersion { version: String },
    #[error("Malformed requirement string: {requirement}")]
    Malformed { requirement: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub constraints: Vec<VersionConstraint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionConstraint {
    pub operator: ComparisonOperator,
    pub version: Version,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Pessimistic,
}

impl Requirement {
    pub fn new(requirements: Vec<impl AsRef<str>>) -> Result<Self, RequirementError> {
        let mut constraints = Vec::new();

        for req in requirements {
            let req_str = req.as_ref().trim();
            if req_str.is_empty() {
                continue;
            }
            constraints.push(Self::parse_requirement(req_str)?);
        }

        // Default to ">= 0" if no constraints
        if constraints.is_empty() {
            let version = Version::new("0").map_err(|_| RequirementError::InvalidVersion {
                version: "0".to_string(),
            })?;
            constraints.push(VersionConstraint {
                operator: ComparisonOperator::GreaterEqual,
                version,
            });
        }

        Ok(Self { constraints })
    }

    pub fn parse(requirement: &str) -> Result<Self, RequirementError> {
        let requirement = requirement.trim();
        if requirement.is_empty() {
            return Err(RequirementError::Empty);
        }
        Self::new(vec![requirement])
    }

    fn parse_requirement(requirement: &str) -> Result<VersionConstraint, RequirementError> {
        let requirement = requirement.trim();

        if requirement.is_empty() {
            return Err(RequirementError::Empty);
        }

        // Try to match operator and version
        let (operator, version_str) =
            if let Some(captures) = Self::extract_operator_and_version(requirement)? {
                captures
            } else {
                // Default to "=" if no operator specified
                (ComparisonOperator::Equal, requirement)
            };

        let version = Version::new(version_str).map_err(|_| RequirementError::InvalidVersion {
            version: version_str.to_string(),
        })?;

        Ok(VersionConstraint { operator, version })
    }

    fn extract_operator_and_version(
        requirement: &str,
    ) -> Result<Option<(ComparisonOperator, &str)>, RequirementError> {
        let requirement = requirement.trim();

        if let Some(stripped) = requirement.strip_prefix(">=") {
            Ok(Some((ComparisonOperator::GreaterEqual, stripped.trim())))
        } else if let Some(stripped) = requirement.strip_prefix("<=") {
            Ok(Some((ComparisonOperator::LessEqual, stripped.trim())))
        } else if let Some(stripped) = requirement.strip_prefix("!=") {
            Ok(Some((ComparisonOperator::NotEqual, stripped.trim())))
        } else if let Some(stripped) = requirement.strip_prefix("~>") {
            Ok(Some((ComparisonOperator::Pessimistic, stripped.trim())))
        } else if let Some(stripped) = requirement.strip_prefix('>') {
            Ok(Some((ComparisonOperator::Greater, stripped.trim())))
        } else if let Some(stripped) = requirement.strip_prefix('<') {
            Ok(Some((ComparisonOperator::Less, stripped.trim())))
        } else if let Some(stripped) = requirement.strip_prefix('=') {
            Ok(Some((ComparisonOperator::Equal, stripped.trim())))
        } else if requirement.starts_with('!') {
            // Handle invalid operators like "! 1"
            Err(RequirementError::InvalidOperator {
                operator: requirement.chars().take(2).collect(),
            })
        } else {
            Ok(None)
        }
    }

    pub fn satisfied_by(&self, version: &Version) -> bool {
        self.constraints
            .iter()
            .all(|constraint| constraint.matches(version))
    }

    pub fn matches(&self, version: &Version) -> bool {
        self.satisfied_by(version)
    }

    pub fn is_latest_version(&self) -> bool {
        // Check if the requirement is just ">= 0"
        self.constraints.len() == 1
            && matches!(
                self.constraints[0].operator,
                ComparisonOperator::GreaterEqual
            )
            && self.constraints[0].version.to_string() == "0"
    }

    pub fn is_prerelease(&self) -> bool {
        // A requirement is prerelease if any of its constraint versions are prerelease
        self.constraints
            .iter()
            .any(|constraint| constraint.version.is_prerelease())
    }
}

impl VersionConstraint {
    pub fn new(operator: ComparisonOperator, version: Version) -> Self {
        Self { operator, version }
    }

    pub fn matches(&self, version: &Version) -> bool {
        match self.operator {
            ComparisonOperator::Equal => version == &self.version,
            ComparisonOperator::NotEqual => version != &self.version,
            ComparisonOperator::Greater => version > &self.version,
            ComparisonOperator::GreaterEqual => version >= &self.version,
            ComparisonOperator::Less => version < &self.version,
            ComparisonOperator::LessEqual => version <= &self.version,
            ComparisonOperator::Pessimistic => {
                // ~> means: version >= self.version && version < self.version.bump()
                version >= &self.version
                    && version < &self.version.bump().unwrap_or_else(|_| self.version.clone())
            }
        }
    }
}

impl std::fmt::Display for ComparisonOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComparisonOperator::Equal => write!(f, "="),
            ComparisonOperator::NotEqual => write!(f, "!="),
            ComparisonOperator::Greater => write!(f, ">"),
            ComparisonOperator::GreaterEqual => write!(f, ">="),
            ComparisonOperator::Less => write!(f, "<"),
            ComparisonOperator::LessEqual => write!(f, "<="),
            ComparisonOperator::Pessimistic => write!(f, "~>"),
        }
    }
}

impl std::fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.operator, self.version)
    }
}

impl std::fmt::Display for Requirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let constraints: Vec<String> = self.constraints.iter().map(|c| c.to_string()).collect();
        write!(f, "{}", constraints.join(", "))
    }
}

impl FromStr for Requirement {
    type Err = RequirementError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Requirement::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(version: &str) -> Version {
        Version::new(version).unwrap()
    }

    fn req(requirement: &str) -> Requirement {
        Requirement::parse(requirement).unwrap()
    }

    #[test]
    fn test_requirement_parsing() {
        // Basic parsing
        insta::assert_debug_snapshot!(req("1.0"), @r###"
        Requirement {
            constraints: [
                VersionConstraint {
                    operator: Equal,
                    version: Version {
                        version: "1.0",
                        segments: [
                            Number(
                                1,
                            ),
                            Number(
                                0,
                            ),
                        ],
                    },
                },
            ],
        }
        "###);

        insta::assert_debug_snapshot!(req("= 1.0"), @r###"
        Requirement {
            constraints: [
                VersionConstraint {
                    operator: Equal,
                    version: Version {
                        version: "1.0",
                        segments: [
                            Number(
                                1,
                            ),
                            Number(
                                0,
                            ),
                        ],
                    },
                },
            ],
        }
        "###);

        insta::assert_debug_snapshot!(req("> 1.0"), @r###"
        Requirement {
            constraints: [
                VersionConstraint {
                    operator: Greater,
                    version: Version {
                        version: "1.0",
                        segments: [
                            Number(
                                1,
                            ),
                            Number(
                                0,
                            ),
                        ],
                    },
                },
            ],
        }
        "###);

        insta::assert_debug_snapshot!(req("~> 1.2"), @r###"
        Requirement {
            constraints: [
                VersionConstraint {
                    operator: Pessimistic,
                    version: Version {
                        version: "1.2",
                        segments: [
                            Number(
                                1,
                            ),
                            Number(
                                2,
                            ),
                        ],
                    },
                },
            ],
        }
        "###);
    }

    #[test]
    fn test_requirement_matching() {
        // Basic matching
        assert!(req("1.0").satisfied_by(&v("1.0")));
        assert!(req("= 1.0").satisfied_by(&v("1.0")));
        assert!(!req("= 1.0").satisfied_by(&v("1.1")));

        // Greater than
        assert!(req("> 1.0").satisfied_by(&v("1.1")));
        assert!(!req("> 1.0").satisfied_by(&v("1.0")));

        // Greater than or equal
        assert!(req(">= 1.0").satisfied_by(&v("1.0")));
        assert!(req(">= 1.0").satisfied_by(&v("1.1")));
        assert!(!req(">= 1.0").satisfied_by(&v("0.9")));

        // Less than
        assert!(req("< 1.0").satisfied_by(&v("0.9")));
        assert!(!req("< 1.0").satisfied_by(&v("1.0")));

        // Less than or equal
        assert!(req("<= 1.0").satisfied_by(&v("1.0")));
        assert!(req("<= 1.0").satisfied_by(&v("0.9")));
        assert!(!req("<= 1.0").satisfied_by(&v("1.1")));

        // Not equal
        assert!(req("!= 1.0").satisfied_by(&v("1.1")));
        assert!(!req("!= 1.0").satisfied_by(&v("1.0")));
    }

    #[test]
    fn test_pessimistic_operator() {
        // ~> 1.4 matches 1.4, 1.5, 1.9 but not 2.0
        assert!(req("~> 1.4").satisfied_by(&v("1.4")));
        assert!(req("~> 1.4").satisfied_by(&v("1.5")));
        assert!(req("~> 1.4").satisfied_by(&v("1.9")));
        assert!(!req("~> 1.4").satisfied_by(&v("2.0")));
        assert!(!req("~> 1.4").satisfied_by(&v("1.3")));

        // ~> 1.4.4 matches 1.4.4, 1.4.5 but not 1.5.0
        assert!(req("~> 1.4.4").satisfied_by(&v("1.4.4")));
        assert!(req("~> 1.4.4").satisfied_by(&v("1.4.5")));
        assert!(!req("~> 1.4.4").satisfied_by(&v("1.5.0")));
        assert!(!req("~> 1.4.4").satisfied_by(&v("1.4.3")));
    }

    #[test]
    fn test_multiple_constraints() {
        let req = Requirement::new(vec![">= 1.4", "<= 1.6", "!= 1.5"]).unwrap();

        assert!(req.satisfied_by(&v("1.4")));
        assert!(req.satisfied_by(&v("1.6")));
        assert!(!req.satisfied_by(&v("1.3")));
        assert!(!req.satisfied_by(&v("1.5")));
        assert!(!req.satisfied_by(&v("1.7")));
    }

    #[test]
    fn test_default_requirement() {
        let req = Requirement::new(vec![""; 0]).unwrap();
        assert_eq!(req.constraints.len(), 1);
        assert_eq!(
            req.constraints[0].operator,
            ComparisonOperator::GreaterEqual
        );
        assert_eq!(req.constraints[0].version, v("0"));
    }

    #[test]
    fn test_prerelease_versions() {
        assert!(req(">= 1.0.0").satisfied_by(&v("1.0.0")));
        assert!(req(">= 1.0.0").satisfied_by(&v("1.0.1")));
        assert!(!req(">= 1.0.0").satisfied_by(&v("1.0.0.a")));
        assert!(req(">= 1.0.0.a").satisfied_by(&v("1.0.0.a")));
        assert!(req(">= 1.0.0.a").satisfied_by(&v("1.0.0")));
    }

    #[test]
    fn test_invalid_requirements() {
        assert!(Requirement::parse("").is_err());
        assert!(Requirement::parse("! 1").is_err());
        assert!(Requirement::parse("= junk").is_err());
        assert!(Requirement::parse("1..2").is_err());
    }

    #[test]
    fn test_is_prerelease() {
        // Regular release versions are not prerelease
        assert!(!req("1.0").is_prerelease());
        assert!(!req("= 1.0.0").is_prerelease());
        assert!(!req("> 1.2.3").is_prerelease());
        assert!(!req(">= 2.0").is_prerelease());
        assert!(!req("< 3.0.0").is_prerelease());
        assert!(!req("<= 1.9.9").is_prerelease());
        assert!(!req("~> 1.4").is_prerelease());

        // Prerelease versions are prerelease
        assert!(req("1.0.alpha").is_prerelease());
        assert!(req("= 1.0.0.beta").is_prerelease());
        assert!(req("> 1.2.3.rc1").is_prerelease());
        assert!(req(">= 2.0.pre").is_prerelease());
        assert!(req("< 3.0.0.dev").is_prerelease());
        assert!(req("<= 1.9.9.a").is_prerelease());
        assert!(req("~> 1.4.alpha.1").is_prerelease());

        // Mixed constraints - prerelease if ANY constraint has prerelease version
        let mixed_req = Requirement::new(vec![">= 1.0", "< 2.0.alpha"]).unwrap();
        assert!(mixed_req.is_prerelease());

        let all_release_req = Requirement::new(vec![">= 1.0", "< 2.0"]).unwrap();
        assert!(!all_release_req.is_prerelease());

        let all_prerelease_req = Requirement::new(vec![">= 1.0.alpha", "< 2.0.beta"]).unwrap();
        assert!(all_prerelease_req.is_prerelease());

        // Default requirement (>= 0) is not prerelease
        let default_req = Requirement::new(vec![""; 0]).unwrap();
        assert!(!default_req.is_prerelease());
    }
}
