//! Detected web/business frameworks grouped by ecosystem.

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// The ecosystem a framework belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ecosystem {
    /// JavaScript / TypeScript
    Javascript,
    /// Python
    Python,
    /// Rust
    Rust,
}

impl Ecosystem {
    /// Human readable ecosystem name.
    pub fn as_str(self) -> &'static str {
        match self {
            Ecosystem::Javascript => "javascript",
            Ecosystem::Python => "python",
            Ecosystem::Rust => "rust",
        }
    }
}

impl fmt::Display for Ecosystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Frameworks the engine can currently detect from manifests and config files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Framework {
    /// Next.js — React meta-framework
    NextJs,
    /// React — UI library
    React,
    /// Express — Node.js web framework
    Express,
    /// NestJS — TypeScript framework
    NestJs,
    /// Vite — build tool / dev server
    Vite,
    /// FastAPI — Python ASGI framework
    FastApi,
    /// Flask — Python WSGI microframework
    Flask,
    /// Django — Python web framework
    Django,
    /// Axum — Rust async web framework
    Axum,
    /// Actix Web — Rust web framework
    Actix,
    /// Rocket — Rust web framework
    Rocket,
}

impl Framework {
    /// Marketing display name used in JSON output (`"Next.js"`, `"FastAPI"`).
    pub fn display_name(self) -> &'static str {
        match self {
            Framework::NextJs => "Next.js",
            Framework::React => "React",
            Framework::Express => "Express",
            Framework::NestJs => "NestJS",
            Framework::Vite => "Vite",
            Framework::FastApi => "FastAPI",
            Framework::Flask => "Flask",
            Framework::Django => "Django",
            Framework::Axum => "Axum",
            Framework::Actix => "Actix",
            Framework::Rocket => "Rocket",
        }
    }

    /// The ecosystem this framework belongs to.
    pub fn ecosystem(self) -> Ecosystem {
        match self {
            Framework::NextJs
            | Framework::React
            | Framework::Express
            | Framework::NestJs
            | Framework::Vite => Ecosystem::Javascript,
            Framework::FastApi | Framework::Flask | Framework::Django => Ecosystem::Python,
            Framework::Axum | Framework::Actix | Framework::Rocket => Ecosystem::Rust,
        }
    }

    /// Iterate all supported frameworks in a stable order.
    pub fn all() -> impl Iterator<Item = Framework> {
        [
            Framework::NextJs,
            Framework::React,
            Framework::Express,
            Framework::NestJs,
            Framework::Vite,
            Framework::FastApi,
            Framework::Flask,
            Framework::Django,
            Framework::Axum,
            Framework::Actix,
            Framework::Rocket,
        ]
        .into_iter()
    }
}

impl fmt::Display for Framework {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

impl FromStr for Framework {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_ascii_lowercase();
        Ok(match normalized.as_str() {
            "next.js" | "nextjs" | "next" => Framework::NextJs,
            "react" => Framework::React,
            "express" => Framework::Express,
            "nestjs" => Framework::NestJs,
            "vite" => Framework::Vite,
            "fastapi" => Framework::FastApi,
            "flask" => Framework::Flask,
            "django" => Framework::Django,
            "axum" => Framework::Axum,
            "actix" | "actix-web" => Framework::Actix,
            "rocket" => Framework::Rocket,
            _ => return Err(()),
        })
    }
}

impl Serialize for Framework {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.display_name())
    }
}

struct FrameworkVisitor;

impl<'de> Visitor<'de> for FrameworkVisitor {
    type Value = Framework;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a supported framework name")
    }

    fn visit_str<E>(self, value: &str) -> Result<Framework, E>
    where
        E: de::Error,
    {
        Framework::from_str(value).map_err(|_| de::Error::unknown_variant(value, &[]))
    }
}

impl<'de> Deserialize<'de> for Framework {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(FrameworkVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_round_trip() {
        for framework in Framework::all() {
            assert_eq!(Framework::from_str(framework.display_name()), Ok(framework));
        }
    }

    #[test]
    fn ecosystems_assigned() {
        assert_eq!(Framework::NextJs.ecosystem(), Ecosystem::Javascript);
        assert_eq!(Framework::FastApi.ecosystem(), Ecosystem::Python);
        assert_eq!(Framework::Axum.ecosystem(), Ecosystem::Rust);
    }

    #[test]
    fn serializes_to_display_name() {
        let json = serde_json::to_string(&Framework::NextJs).unwrap();
        assert_eq!(json, "\"Next.js\"");
    }
}
