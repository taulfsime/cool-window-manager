use anyhow::Result;
use core_foundation::base::TCFType;

/// Check if the application has accessibility permissions
pub fn is_trusted() -> bool {
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    unsafe {
        // AXIsProcessTrustedWithOptions with no prompt
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::false_value();
        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef())
    }
}

/// Check permissions and prompt user to grant if not trusted
pub fn check_and_prompt() -> bool {
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    unsafe {
        // AXIsProcessTrustedWithOptions with prompt
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef())
    }
}

pub fn print_permission_status() -> Result<()> {
    if is_trusted() {
        println!("✓ Accessibility permissions granted");
        Ok(())
    } else {
        println!("✗ Accessibility permissions not granted");
        println!();
        println!("To grant permissions:");
        println!("1. Open System Settings");
        println!("2. Go to Privacy & Security > Accessibility");
        println!("3. Enable access for this application");
        println!();
        println!("Run this command again to prompt for permissions:");
        println!("  cwm check-permissions --prompt");
        Ok(())
    }
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: core_foundation::dictionary::CFDictionaryRef)
        -> bool;
}
