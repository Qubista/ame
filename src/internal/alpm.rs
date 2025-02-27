use std::{
    fmt::{Display, Formatter},
    path::Path,
    path::PathBuf,
};

use alpm::SigLevel;
use alpm_utils::alpm_with_conf;
use pacmanconf::Config;

#[derive(Debug)]
pub enum Error {
    Alpm(alpm::Error),
    Pacmanconf(pacmanconf::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alpm(e) => Display::fmt(e, f),
            Self::Pacmanconf(e) => Display::fmt(e, f),
        }
    }
}

impl std::error::Error for Error {}

impl From<alpm::Error> for Error {
    fn from(err: alpm::Error) -> Self {
        Error::Alpm(err)
    }
}

impl From<pacmanconf::Error> for Error {
    fn from(err: pacmanconf::Error) -> Self {
        Error::Pacmanconf(err)
    }
}

pub enum PackageFrom {
    #[allow(dead_code)]
    LocalDb(String),

    #[allow(dead_code)]
    SyncDb(String),

    File(PathBuf),
}

pub enum AlpmPackage<'a> {
    Found(&'a alpm::Package),
    Loaded(alpm::LoadedPackage<'a>),
}

impl AlpmPackage<'_> {
    pub fn name(&self) -> &str {
        match self {
            Self::Found(p) => p.name(),
            Self::Loaded(p) => p.name(),
        }
    }
}

pub struct Alpm(alpm::Alpm);

impl Alpm {
    #[tracing::instrument(level = "trace")]
    pub fn new() -> Result<Self, Error> {
        let config = Config::from_file(Path::new("/etc/pacman.conf"))?;
        let alpm = alpm_with_conf(&config)?;
        tracing::debug!("Initialized alpm handler");
        Ok(Self(alpm))
    }

    pub fn load(&self, pkg: PackageFrom) -> Result<AlpmPackage, Error> {
        match pkg {
            PackageFrom::LocalDb(name) => {
                let db = self.0.localdb();
                let package = db.pkg(name)?;
                Ok(AlpmPackage::Found(package))
            }
            PackageFrom::SyncDb(name) => {
                let package = self
                    .0
                    .syncdbs()
                    .find_satisfier(name)
                    .ok_or(Error::Alpm(alpm::Error::PkgNotFound))?;
                Ok(AlpmPackage::Found(package))
            }
            PackageFrom::File(path) => {
                let package = self
                    .0
                    .pkg_load(path.to_str().unwrap(), true, SigLevel::NONE)?;
                Ok(AlpmPackage::Loaded(package))
            }
        }
    }

    pub fn handler(&self) -> &alpm::Alpm {
        &self.0
    }

    pub fn group_packages(&self, group_name: String) -> Result<Vec<AlpmPackage>, Error> {
        let mut packages = Vec::new();
        for db in self.0.syncdbs() {
            if let Ok(group) = db.group(group_name.clone()) {
                for package in group.packages() {
                    packages.push(AlpmPackage::Found(package));
                }
            }
        }
        if packages.is_empty() {
            return Err(Error::Alpm(alpm::Error::PkgNotFound));
        }
        Ok(packages)
    }
}
