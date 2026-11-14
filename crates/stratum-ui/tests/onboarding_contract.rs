//! Window-free contracts for the first-run onboarding wizard.
//!
//! The i18n gate walks every key the wizard renders against all eight catalogs
//! and the fixture flows exercise the provider calls the finish step makes.

use stratum_i18n::{t, Locale};
use stratum_ui::provider::{FixtureProvider, PanelData, SettingsData};
use stratum_ui::shell::ONBOARDING_I18N_KEYS;
use stratum_ui::Shell;

#[test]
fn every_onboarding_key_resolves_in_all_catalogs() {
    for locale in Locale::ALL {
        for key in ONBOARDING_I18N_KEYS {
            assert_ne!(t(locale, key), *key, "{key} missing in {}", locale.code());
        }
    }
}

#[test]
fn a_fresh_fixture_vault_is_not_onboarding_pending_by_default() {
    let provider = FixtureProvider::new();
    assert!(!provider.onboarding_pending());
    let shell = Shell::new(std::sync::Arc::new(provider));
    assert!(!shell.onboarding_open());
}

#[test]
fn resetting_onboarding_reopens_the_wizard_on_the_next_shell() {
    let provider = std::sync::Arc::new(FixtureProvider::new());
    provider
        .set_onboarding_pending(true)
        .expect("reset succeeds");
    assert!(provider.onboarding_pending());
    let shell = Shell::new(provider.clone());
    assert!(shell.onboarding_open());
    assert_eq!(
        shell.onboarding().step(),
        stratum_ui::OnboardingStep::Welcome
    );

    provider
        .mark_onboarding_done()
        .expect("completion succeeds");
    assert!(!provider.onboarding_pending());
}

#[test]
fn the_starter_extras_are_available_for_the_finish_step() {
    let provider = FixtureProvider::new();
    let templates = provider
        .install_starter_templates()
        .expect("templates install");
    assert_eq!(templates.len(), 3);
    let sample = provider.install_starter_sample().expect("sample seeds");
    assert_eq!(sample.len(), 4);
}

#[test]
fn wizard_steps_parse_from_the_capture_flags() {
    use stratum_ui::OnboardingStep;
    assert_eq!(
        OnboardingStep::from_env("welcome"),
        Some(OnboardingStep::Welcome)
    );
    assert_eq!(
        OnboardingStep::from_env("templates"),
        Some(OnboardingStep::Templates)
    );
    assert_eq!(
        OnboardingStep::from_env("sample"),
        Some(OnboardingStep::Sample)
    );
    assert_eq!(
        OnboardingStep::from_env("finish"),
        Some(OnboardingStep::Finish)
    );
    assert_eq!(OnboardingStep::from_env("nope"), None);
}
