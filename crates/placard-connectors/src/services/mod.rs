pub(crate) mod meta;

mod amo;
mod ansible;
mod appveyor;
mod archlinux;
mod aur;
mod azure_devops;
mod bitbucket;
mod bitrise;
mod bluesky;
mod bower;
mod bstats;
mod bugzilla;
mod buildkite;
mod bundlejs;
mod bundlephobia;
mod cdnjs;
mod chocolatey;
mod chrome_web_store;
mod cii_best_practices;
mod circleci;
mod clearlydefined;
mod clojars;
mod cocoapods;
mod codacy;
mod codecov;
mod codefactor;
mod coderabbit;
mod conda;
mod cookbook;
mod coveralls;
mod coverity;
mod cpan;
mod cran;
mod crates;
mod ctan;
mod debian;
mod depfu;
mod deps_rs;
mod discourse;
mod docker;
mod docsrs;
mod dub;
mod dynamic;
mod eclipse_marketplace;
mod ecologi;
mod elm_package;
mod endpoint;
mod f_droid;
mod factorio_mod_portal;
mod fedora;
mod feedz;
mod flathub;
mod freecodecamp;
mod galaxytoolshed;
mod gem;
mod gerrit;
mod gitea;
mod github;
mod gitlab;
mod gnome_extensions;
mod gradle_plugin_portal;
mod greasyfork;
mod hackage;
mod hackernews;
mod hangar;
mod hexpm;
mod homebrew;
mod hsts;
mod itunes;
mod jenkins;
mod jetbrains;
mod jitpack;
mod jsdelivr;
mod jsr;
mod keybase;
mod lemmy;
mod liberapay;
mod librariesio;
mod macports;
mod mastodon;
mod matrix;
mod maven_central;
mod maven_metadata;
mod mbin;
mod modrinth;
mod myget;
mod netlify;
mod node;
mod nodeping;
mod npm;
mod npm_stat;
mod npms_io;
mod nuget;
mod nycrc;
mod open_vsx;
mod opencollective;
mod ossf_scorecard;
mod osslifecycle;
mod packagecontrol;
mod packagist;
mod pingpong;
mod piwheels;
pub(crate) mod placard;
mod polymart;
mod powershellgallery;
#[path = "pub/mod.rs"]
mod pub_dir;
mod pulsar;
mod puppetforge;
mod pypi;
mod python;
mod raycast;
mod readthedocs;
mod repology;
mod reproducible_central;
mod resharper;
mod reuse;
mod revolt;
mod scoop;
mod scrutinizer;
mod sdkman;
mod sonar;
mod sourceforge;
mod sourcegraph;
mod spack;
mod spiget;
mod stackexchange;
mod swagger;
mod teamcity;
mod terraform;
mod testspace;
mod thunderstore;
mod travis;
mod treeware;
mod ubuntu;
mod uptimeobserver;
mod vaadin_directory;
mod vcpkg;
mod voxelshop;
mod w3c;
mod weblate;
mod website;
mod whatpulse;
mod wordpress;

use crate::Fetcher;
use std::collections::HashMap;

pub fn all_presets() -> impl Iterator<Item = &'static meta::PresetMeta> {
    const ALL: &[&[meta::PresetMeta]] = &[
        amo::PRESETS,
        ansible::PRESETS,
        appveyor::PRESETS,
        archlinux::PRESETS,
        aur::PRESETS,
        bitbucket::PRESETS,
        bitrise::PRESETS,
        bluesky::PRESETS,
        bower::PRESETS,
        bstats::PRESETS,
        buildkite::PRESETS,
        bugzilla::PRESETS,
        bundlejs::PRESETS,
        bundlephobia::PRESETS,
        clearlydefined::PRESETS,
        clojars::PRESETS,
        cdnjs::PRESETS,
        chocolatey::PRESETS,
        cii_best_practices::PRESETS,
        circleci::PRESETS,
        cocoapods::PRESETS,
        codacy::PRESETS,
        codecov::PRESETS,
        codefactor::PRESETS,
        coderabbit::PRESETS,
        cookbook::PRESETS,
        coveralls::PRESETS,
        coverity::PRESETS,
        cpan::PRESETS,
        cran::PRESETS,
        ctan::PRESETS,
        crates::PRESETS,
        azure_devops::PRESETS,
        chrome_web_store::PRESETS,
        conda::PRESETS,
        debian::PRESETS,
        depfu::PRESETS,
        deps_rs::PRESETS,
        discourse::PRESETS,
        docsrs::PRESETS,
        docker::PRESETS,
        dub::PRESETS,
        ecologi::PRESETS,
        elm_package::PRESETS,
        endpoint::PRESETS,
        factorio_mod_portal::PRESETS,
        f_droid::PRESETS,
        fedora::PRESETS,
        feedz::PRESETS,
        flathub::PRESETS,
        freecodecamp::PRESETS,
        galaxytoolshed::PRESETS,
        gerrit::PRESETS,
        gradle_plugin_portal::PRESETS,
        gnome_extensions::PRESETS,
        hackage::PRESETS,
        hackernews::PRESETS,
        hexpm::PRESETS,
        hsts::PRESETS,
        dynamic::PRESETS,
        gem::PRESETS,
        greasyfork::PRESETS,
        hangar::PRESETS,
        homebrew::PRESETS,
        eclipse_marketplace::PRESETS,
        gitea::PRESETS,
        github::PRESETS,
        gitlab::PRESETS,
        itunes::PRESETS,
        jenkins::PRESETS,
        jetbrains::PRESETS,
        jitpack::PRESETS,
        jsdelivr::PRESETS,
        jsr::PRESETS,
        keybase::PRESETS,
        lemmy::PRESETS,
        liberapay::PRESETS,
        librariesio::PRESETS,
        macports::PRESETS,
        mastodon::PRESETS,
        matrix::PRESETS,
        maven_central::PRESETS,
        maven_metadata::PRESETS,
        mbin::PRESETS,
        modrinth::PRESETS,
        myget::PRESETS,
        netlify::PRESETS,
        npms_io::PRESETS,
        npm_stat::PRESETS,
        nuget::PRESETS,
        nycrc::PRESETS,
        node::PRESETS,
        nodeping::PRESETS,
        npm::PRESETS,
        opencollective::PRESETS,
        polymart::PRESETS,
        open_vsx::PRESETS,
        ossf_scorecard::PRESETS,
        osslifecycle::PRESETS,
        packagecontrol::PRESETS,
        pingpong::PRESETS,
        piwheels::PRESETS,
        placard::PRESETS,
        powershellgallery::PRESETS,
        pulsar::PRESETS,
        python::PRESETS,
        repology::PRESETS,
        reproducible_central::PRESETS,
        resharper::PRESETS,
        reuse::PRESETS,
        revolt::PRESETS,
        packagist::PRESETS,
        pub_dir::PRESETS,
        puppetforge::PRESETS,
        pypi::PRESETS,
        raycast::PRESETS,
        readthedocs::PRESETS,
        scoop::PRESETS,
        scrutinizer::PRESETS,
        sdkman::PRESETS,
        sonar::PRESETS,
        sourceforge::PRESETS,
        sourcegraph::PRESETS,
        spack::PRESETS,
        spiget::PRESETS,
        stackexchange::PRESETS,
        swagger::PRESETS,
        teamcity::PRESETS,
        terraform::PRESETS,
        testspace::PRESETS,
        thunderstore::PRESETS,
        travis::PRESETS,
        treeware::PRESETS,
        ubuntu::PRESETS,
        uptimeobserver::PRESETS,
        vaadin_directory::PRESETS,
        vcpkg::PRESETS,
        voxelshop::PRESETS,
        w3c::PRESETS,
        weblate::PRESETS,
        website::PRESETS,
        whatpulse::PRESETS,
        wordpress::PRESETS,
    ];
    ALL.iter().copied().flatten()
}

pub(crate) fn resolve_preset(
    name: &str,
    params: &HashMap<String, String>,
    fetcher: &dyn Fetcher,
) -> Result<String, String> {
    all_presets()
        .find(|p| p.preset == name)
        .ok_or_else(|| format!("unknown preset '{name}'"))
        .and_then(|p| (p.resolve)(params, fetcher))
}

/// Rejects parameter values that could break out of a URL path segment
/// when interpolated into a service's fixed request-URL template --
/// presets never take an arbitrary URL from the caller, only small
/// identifiers like a repo owner/name, so this just needs to rule out
/// path/query/fragment-breaking characters.
pub(crate) fn validate_path_param<'a>(name: &str, value: &'a str) -> Result<&'a str, String> {
    if value.is_empty() {
        return Err(format!("'{name}' parameter must not be empty"));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(format!("'{name}' parameter contains disallowed characters"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_presets() {
        let fetcher_err = "unknown preset 'not-a-real-preset'";
        struct Unused;
        impl Fetcher for Unused {
            fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
                unreachable!("should never be called for an unknown preset")
            }
        }
        let err = resolve_preset("not-a-real-preset", &HashMap::new(), &Unused).unwrap_err();
        assert_eq!(err, fetcher_err);
    }

    #[test]
    fn validate_path_param_rejects_path_breaking_characters() {
        assert!(validate_path_param("owner", "shdwmtr").is_ok());
        assert!(validate_path_param("owner", "../etc/passwd").is_err());
        assert!(validate_path_param("owner", "a/b").is_err());
        assert!(validate_path_param("owner", "a?b=c").is_err());
        assert!(validate_path_param("owner", "").is_err());
    }

    #[test]
    fn registry_has_the_expected_preset_count() {
        assert_eq!(all_presets().count(), 384);
    }

    #[test]
    fn registry_has_no_duplicate_preset_names() {
        let mut seen = std::collections::HashSet::new();
        for p in all_presets() {
            assert!(seen.insert(p.preset), "duplicate preset name: {}", p.preset);
        }
    }
}
