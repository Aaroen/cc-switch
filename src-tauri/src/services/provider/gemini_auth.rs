//! Gemini authentication type detection
//!
//! Detects whether a Gemini provider uses PackyCode API Key, Google OAuth, or generic API Key.

use crate::error::AppError;
use crate::provider::Provider;
use crate::settings;

/// Gemini authentication type enumeration
///
/// Used to optimize performance by avoiding repeated provider type detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GeminiAuthType {
    /// PackyCode provider (uses API Key)
    Packycode,
    /// Google Official (uses OAuth)
    GoogleOfficial,
    /// Generic Gemini provider (uses API Key)
    Generic,
}

// Authentication type constants
const PACKYCODE_SECURITY_SELECTED_TYPE: &str = "gemini-api-key";
const GOOGLE_OAUTH_SECURITY_SELECTED_TYPE: &str = "oauth-personal";

// Partner Promotion Key constants
const PACKYCODE_PARTNER_KEY: &str = "packycode";
const GOOGLE_OFFICIAL_PARTNER_KEY: &str = "google-official";

// PackyCode keyword constants
const PACKYCODE_KEYWORDS: [&str; 3] = ["packycode", "packyapi", "packy"];

/// Detect Gemini provider authentication type
///
/// One-time detection to avoid repeated calls to `is_packycode_gemini` and `is_google_official_gemini`.
///
/// # Returns
///
/// - `GeminiAuthType::GoogleOfficial`: Google official, uses OAuth
/// - `GeminiAuthType::Packycode`: PackyCode provider, uses API Key
/// - `GeminiAuthType::Generic`: Other generic providers, uses API Key
pub(crate) fn detect_gemini_auth_type(provider: &Provider) -> GeminiAuthType {
    // Priority 1: Check partner_promotion_key (most reliable)
    if let Some(key) = provider
        .meta
        .as_ref()
        .and_then(|meta| meta.partner_promotion_key.as_deref())
    {
        if key.eq_ignore_ascii_case(GOOGLE_OFFICIAL_PARTNER_KEY) {
            return GeminiAuthType::GoogleOfficial;
        }
        if key.eq_ignore_ascii_case(PACKYCODE_PARTNER_KEY) {
            return GeminiAuthType::Packycode;
        }
    }

    // Priority 2: Check Google Official (name matching)
    let name_lower = provider.name.to_ascii_lowercase();
    if name_lower == "google" || name_lower.starts_with("google ") {
        return GeminiAuthType::GoogleOfficial;
    }

    // Priority 3: Check PackyCode keywords
    if contains_packycode_keyword(&provider.name) {
        return GeminiAuthType::Packycode;
    }

    if let Some(site) = provider.website_url.as_deref() {
        if contains_packycode_keyword(site) {
            return GeminiAuthType::Packycode;
        }
    }

    if let Some(base_url) = provider
        .settings_config
        .pointer("/env/GOOGLE_GEMINI_BASE_URL")
        .and_then(|v| v.as_str())
    {
        if contains_packycode_keyword(base_url) {
            return GeminiAuthType::Packycode;
        }
    }

    GeminiAuthType::Generic
}

/// Check if string contains PackyCode related keywords (case-insensitive)
///
/// Keyword list: ["packycode", "packyapi", "packy"]
fn contains_packycode_keyword(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    PACKYCODE_KEYWORDS
        .iter()
        .any(|keyword| lower.contains(keyword))
}

/// Detect if provider is PackyCode Gemini (uses API Key authentication)
///
/// PackyCode is an official partner requiring special security configuration.
///
/// # Detection Rules (priority from high to low)
///
/// 1. **Partner Promotion Key** (most reliable):
///    - `provider.meta.partner_promotion_key == "packycode"`
///
/// 2. **Provider name**:
///    - Name contains "packycode", "packyapi" or "packy" (case-insensitive)
///
/// 3. **Website URL**:
///    - `provider.website_url` contains keywords
///
/// 4. **Base URL**:
///    - `settings_config.env.GOOGLE_GEMINI_BASE_URL` contains keywords
///
/// # Why multiple detection methods
///
/// - Users may manually create providers without `partner_promotion_key`
/// - Meta fields may be modified after copying from presets
/// - Ensure all PackyCode providers get correct security flags
#[allow(dead_code)]
pub(crate) fn is_packycode_gemini(provider: &Provider) -> bool {
    // Strategy 1: Check partner_promotion_key (most reliable)
    if provider
        .meta
        .as_ref()
        .and_then(|meta| meta.partner_promotion_key.as_deref())
        .is_some_and(|key| key.eq_ignore_ascii_case(PACKYCODE_PARTNER_KEY))
    {
        return true;
    }

    // Strategy 2: Check provider name
    if contains_packycode_keyword(&provider.name) {
        return true;
    }

    // Strategy 3: Check website URL
    if let Some(site) = provider.website_url.as_deref() {
        if contains_packycode_keyword(site) {
            return true;
        }
    }

    // Strategy 4: Check Base URL
    if let Some(base_url) = provider
        .settings_config
        .pointer("/env/GOOGLE_GEMINI_BASE_URL")
        .and_then(|v| v.as_str())
    {
        if contains_packycode_keyword(base_url) {
            return true;
        }
    }

    false
}

/// Detect if provider is Google Official Gemini (uses OAuth authentication)
///
/// Google Official Gemini uses OAuth personal authentication, no API Key needed.
///
/// # Detection Rules (priority from high to low)
///
/// 1. **Partner Promotion Key** (most reliable):
///    - `provider.meta.partner_promotion_key == "google-official"`
///
/// 2. **Provider name**:
///    - Name equals "google" (case-insensitive)
///    - Or name starts with "google " (e.g., "Google Official")
///
/// # OAuth vs API Key
///
/// - **OAuth mode**: `security.auth.selectedType = "oauth-personal"`
///   - User needs to login via browser with Google account
///   - No API Key needed in `.env` file
///
/// - **API Key mode**: `security.auth.selectedType = "gemini-api-key"`
///   - Used for third-party relay services (like PackyCode)
///   - Requires `GEMINI_API_KEY` in `.env` file
#[allow(dead_code)]
pub(crate) fn is_google_official_gemini(provider: &Provider) -> bool {
    // Strategy 1: Check partner_promotion_key (most reliable)
    if provider
        .meta
        .as_ref()
        .and_then(|meta| meta.partner_promotion_key.as_deref())
        .is_some_and(|key| key.eq_ignore_ascii_case(GOOGLE_OFFICIAL_PARTNER_KEY))
    {
        return true;
    }

    // Strategy 2: Check name matching (fallback)
    let name_lower = provider.name.to_ascii_lowercase();
    name_lower == "google" || name_lower.starts_with("google ")
}

/// Ensure PackyCode Gemini provider security flag is correctly set
///
/// PackyCode is an official partner using API Key authentication mode.
///
/// # Why write to two settings.json files
///
/// 1. **`~/.cc-switch/settings.json`** (application-level config):
///    - CC-Switch application global settings
///    - Ensures app knows current authentication type
///    - Used for UI display and other app logic
///
/// 2. **`~/.gemini/settings.json`** (Gemini client config):
///    - Configuration file read by Gemini CLI client
///    - Directly affects Gemini client authentication behavior
///    - Ensures Gemini uses correct authentication method to connect API
///
/// # Value set
///
/// ```json
/// {
///   "security": {
///     "auth": {
///       "selectedType": "gemini-api-key"
///     }
///   }
/// }
/// ```
///
/// # Error handling
///
/// If provider is not PackyCode, function returns `Ok(())` immediately without any operation.
pub(crate) fn ensure_packycode_security_flag(provider: &Provider) -> Result<(), AppError> {
    if !is_packycode_gemini(provider) {
        return Ok(());
    }

    // Write to application-level settings.json (~/.cc-switch/settings.json)
    settings::ensure_security_auth_selected_type(PACKYCODE_SECURITY_SELECTED_TYPE)?;

    // Write to Gemini directory settings.json (~/.gemini/settings.json)
    use crate::gemini_config::write_packycode_settings;
    write_packycode_settings()?;

    Ok(())
}

/// Ensure Google Official Gemini provider security flag is correctly set (OAuth mode)
///
/// Google Official Gemini uses OAuth personal authentication, no API Key needed.
///
/// # Why write to two settings.json files
///
/// Same as `ensure_packycode_security_flag`, need to configure both app-level and client-level settings.
///
/// # Value set
///
/// ```json
/// {
///   "security": {
///     "auth": {
///       "selectedType": "oauth-personal"
///     }
///   }
/// }
/// ```
///
/// # OAuth authentication flow
///
/// 1. User switches to Google Official provider
/// 2. CC-Switch sets `selectedType = "oauth-personal"`
/// 3. User's first use of Gemini CLI will auto-open browser for OAuth login
/// 4. After successful login, credentials saved in Gemini credential store
/// 5. Subsequent requests auto-use saved credentials
///
/// # Error handling
///
/// If provider is not Google Official, function returns `Ok(())` immediately without any operation.
pub(crate) fn ensure_google_oauth_security_flag(provider: &Provider) -> Result<(), AppError> {
    if !is_google_official_gemini(provider) {
        return Ok(());
    }

    // Write to application-level settings.json (~/.cc-switch/settings.json)
    settings::ensure_security_auth_selected_type(GOOGLE_OAUTH_SECURITY_SELECTED_TYPE)?;

    // Write to Gemini directory settings.json (~/.gemini/settings.json)
    use crate::gemini_config::write_google_oauth_settings;
    write_google_oauth_settings()?;

    Ok(())
}
