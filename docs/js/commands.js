// command handlers for the terminal

const Commands = (function() {
  'use strict';

  const GITHUB_URL = 'https://github.com/taulfsime/cool-window-manager';
  const INSTALL_CMD = 'curl -fsSL https://raw.githubusercontent.com/taulfsime/cool-window-manager/main/install.sh | sh';

  // command registry
  const commands = {
    help: cmdHelp,
    install: cmdInstall,
    demo: cmdDemo,
    features: cmdFeatures,
    why: cmdWhy,
    github: cmdGithub,
    clear: cmdClear,
    cwm: cmdCwm,
    about: cmdAbout,
    version: cmdVersion
  };

  // execute a command string
  function execute(input) {
    const parts = input.trim().split(/\s+/);
    const cmd = parts[0].toLowerCase();
    const args = parts.slice(1);

    if (commands[cmd]) {
      commands[cmd](args);
    } else {
      Terminal.writeLine(`command not found: ${cmd}`, 'error');
      Terminal.writeLine("type 'help' for available commands", 'info');
    }
  }

  // help command
  function cmdHelp() {
    Terminal.writeLine('');
    Terminal.writeLine('Available commands:', 'highlight');
    Terminal.writeLine('');
    
    const helpItems = [
      ['help', 'Show this help message'],
      ['install', 'Show installation command'],
      ['demo', 'Run interactive demo'],
      ['features', 'List cwm features'],
      ['why', 'Why use cwm?'],
      ['about', 'About cwm'],
      ['github', 'Open GitHub repository'],
      ['clear', 'Clear terminal'],
      ['', ''],
      ['cwm --help', 'Show cwm usage'],
      ['cwm focus --help', 'Focus command help'],
      ['cwm maximize --help', 'Maximize command help'],
      ['cwm move-display --help', 'Move-display command help'],
      ['cwm resize --help', 'Resize command help']
    ];

    helpItems.forEach(([cmd, desc]) => {
      if (cmd === '') {
        Terminal.writeLine('');
      } else {
        const paddedCmd = cmd.padEnd(24);
        Terminal.writeHtml(
          `<span class="help-cmd">${paddedCmd}</span><span class="help-desc">${desc}</span>`
        );
      }
    });
    
    Terminal.writeLine('');
  }

  // install command
  function cmdInstall() {
    Terminal.writeLine('');
    Terminal.writeLine('Quick Install (recommended):', 'highlight');
    Terminal.writeLine('');
    Terminal.writeHtml(`<div class="code-block">${escapeHtml(INSTALL_CMD)}</div>`);
    
    // add copy button
    const copyBtn = document.createElement('button');
    copyBtn.className = 'copy-button';
    copyBtn.innerHTML = '📋 Copy to clipboard';
    copyBtn.onclick = async () => {
      try {
        await navigator.clipboard.writeText(INSTALL_CMD);
        copyBtn.innerHTML = '✓ Copied!';
        copyBtn.classList.add('copied');
        setTimeout(() => {
          copyBtn.innerHTML = '📋 Copy to clipboard';
          copyBtn.classList.remove('copied');
        }, 2000);
      } catch (e) {
        Terminal.writeLine('Failed to copy to clipboard', 'error');
      }
    };
    
    const output = document.getElementById('terminal-output');
    output.appendChild(copyBtn);
    
    Terminal.writeLine('');
    Terminal.writeLine('Or install specific channel:', 'info');
    Terminal.writeLine('  --stable  Well-tested releases (default)');
    Terminal.writeLine('  --beta    Preview features');
    Terminal.writeLine('  --dev     Latest features');
    Terminal.writeLine('');
  }

  // demo command
  function cmdDemo() {
    Terminal.writeLine('');
    Terminal.writeLine('Starting demo...', 'info');
    Terminal.writeLine('');
    
    if (typeof Demo !== 'undefined') {
      Demo.run();
    } else {
      Terminal.writeLine('Demo module not loaded', 'error');
    }
  }

  // features command
  function cmdFeatures() {
    Terminal.writeLine('');
    Terminal.writeLine('cwm Features:', 'highlight');
    Terminal.writeLine('');
    
    const features = [
      ['Focus windows by app name', 'cwm focus --app Slack'],
      ['Fuzzy matching', '"slck" matches "Slack"'],
      ['Regex matching', '"/chrome|safari/i" matches browsers'],
      ['Match by window title', 'Focus Chrome tab by title'],
      ['Maximize windows', 'cwm maximize'],
      ['Move between displays', 'cwm move-display next'],
      ['Resize to percentage', 'cwm resize 75'],
      ['Global hotkeys', 'Ctrl+Alt+S to focus Slack'],
      ['App launch rules', 'Auto-maximize Terminal on launch'],
      ['Spotlight integration', 'Search "cwm: Focus Safari"'],
      ['Auto-updates', 'Stay up to date automatically']
    ];

    features.forEach(([feature, example]) => {
      Terminal.writeHtml(`<div class="feature-item"><span>${feature}</span></div>`);
      Terminal.writeLine(`    ${example}`, 'info');
    });
    
    Terminal.writeLine('');
  }

  // why command
  function cmdWhy() {
    Terminal.writeLine('');
    Terminal.writeLine('Why cwm?', 'highlight');
    Terminal.writeLine('');
    Terminal.writeLine('macOS window management is tedious:');
    Terminal.writeLine('  • Cmd+Tab cycles through ALL windows');
    Terminal.writeLine('  • No quick way to focus a specific app');
    Terminal.writeLine('  • Moving windows between monitors is slow');
    Terminal.writeLine('  • No automation for window placement');
    Terminal.writeLine('');
    Terminal.writeLine('cwm solves this:', 'success');
    Terminal.writeLine('  • Focus any app instantly by name');
    Terminal.writeLine('  • Fuzzy matching - no exact spelling needed');
    Terminal.writeLine('  • Global hotkeys for your most-used apps');
    Terminal.writeLine('  • Automatic rules when apps launch');
    Terminal.writeLine('  • Works with Spotlight for quick access');
    Terminal.writeLine('');
    Terminal.writeLine('Built with Rust for speed and reliability.', 'info');
    Terminal.writeLine('');
  }

  // about command
  function cmdAbout() {
    Terminal.writeLine('');
    Terminal.writeHtml('<span class="highlight">cwm</span> - Cool Window Manager');
    Terminal.writeLine('');
    Terminal.writeLine('A macOS window manager with CLI and global hotkeys.');
    Terminal.writeLine('Manage windows by app name with fuzzy matching.');
    Terminal.writeLine('');
    Terminal.writeLine('Author: taulfsime');
    Terminal.writeLine(`Repository: ${GITHUB_URL}`);
    Terminal.writeLine('License: MIT');
    Terminal.writeLine('');
  }

  // version command
  function cmdVersion() {
    Terminal.writeLine('');
    Terminal.writeLine('cwm (website demo)', 'info');
    Terminal.writeLine('For actual version, install cwm and run: cwm version');
    Terminal.writeLine('');
  }

  // github command
  function cmdGithub() {
    Terminal.writeLine('');
    Terminal.writeLine(`Opening ${GITHUB_URL}`, 'info');
    window.open(GITHUB_URL, '_blank');
    Terminal.writeLine('');
  }

  // clear command
  function cmdClear() {
    Terminal.clear();
    if (typeof Preview !== 'undefined') {
      Preview.reset();
    }
  }

  // cwm command (simulated)
  function cmdCwm(args) {
    if (args.length === 0 || args[0] === '--help' || args[0] === '-h') {
      showCwmHelp();
      return;
    }

    const subcommand = args[0].toLowerCase();
    const subArgs = args.slice(1);

    switch (subcommand) {
      case 'focus':
        cmdCwmFocus(subArgs);
        break;
      case 'maximize':
        cmdCwmMaximize(subArgs);
        break;
      case 'move-display':
        cmdCwmMoveDisplay(subArgs);
        break;
      case 'resize':
        cmdCwmResize(subArgs);
        break;
      case 'version':
        cmdVersion();
        break;
      default:
        Terminal.writeLine(`Unknown subcommand: ${subcommand}`, 'error');
        Terminal.writeLine("Run 'cwm --help' for usage", 'info');
    }
  }

  function showCwmHelp() {
    Terminal.writeLine('');
    Terminal.writeLine('cwm - Cool Window Manager for macOS', 'highlight');
    Terminal.writeLine('');
    Terminal.writeLine('Usage: cwm <command> [options]');
    Terminal.writeLine('');
    Terminal.writeLine('Commands:');
    Terminal.writeLine('  focus          Focus an application window');
    Terminal.writeLine('  maximize       Maximize a window');
    Terminal.writeLine('  move-display   Move window to another display');
    Terminal.writeLine('  resize         Resize window to percentage');
    Terminal.writeLine('  list-apps      List running applications');
    Terminal.writeLine('  list-displays  List available displays');
    Terminal.writeLine('  daemon         Manage background daemon');
    Terminal.writeLine('  config         Manage configuration');
    Terminal.writeLine('  install        Install cwm to PATH');
    Terminal.writeLine('  update         Update to latest version');
    Terminal.writeLine('  version        Show version info');
    Terminal.writeLine('');
    Terminal.writeLine("Run 'cwm <command> --help' for command details", 'info');
    Terminal.writeLine('');
  }

  // parse regex pattern from /pattern/ or /pattern/i syntax
  function parseRegexPattern(query) {
    if (!query.startsWith('/')) return null;
    
    if (query.endsWith('/i') && query.length > 3) {
      return { pattern: query.slice(1, -2), caseInsensitive: true };
    } else if (query.endsWith('/') && query.length > 2) {
      return { pattern: query.slice(1, -1), caseInsensitive: false };
    }
    return null;
  }

  function cmdCwmFocus(args) {
    if (args.includes('--help') || args.includes('-h')) {
      Terminal.writeLine('');
      Terminal.writeLine('cwm focus - Focus an application window', 'highlight');
      Terminal.writeLine('');
      Terminal.writeLine('Usage: cwm focus --app <name>');
      Terminal.writeLine('');
      Terminal.writeLine('Options:');
      Terminal.writeLine('  --app, -a <name>  Target app name (fuzzy/regex matched)');
      Terminal.writeLine('  --launch          Launch app if not running');
      Terminal.writeLine('  --no-launch       Never launch app');
      Terminal.writeLine('  --verbose, -v     Show matching details');
      Terminal.writeLine('');
      Terminal.writeLine('Matching:');
      Terminal.writeLine('  Exact match       "Safari" matches Safari');
      Terminal.writeLine('  Prefix match      "Saf" matches Safari');
      Terminal.writeLine('  Regex match       "/^Google/" matches Google Chrome');
      Terminal.writeLine('  Fuzzy match       "slck" matches Slack');
      Terminal.writeLine('');
      Terminal.writeLine('Regex syntax:');
      Terminal.writeLine('  /pattern/         Case-sensitive regex');
      Terminal.writeLine('  /pattern/i        Case-insensitive regex');
      Terminal.writeLine('');
      Terminal.writeLine('Examples:');
      Terminal.writeLine('  cwm focus --app Safari');
      Terminal.writeLine('  cwm focus --app "slck"           # fuzzy match');
      Terminal.writeLine('  cwm focus --app "New Tab"        # match by title');
      Terminal.writeLine('  cwm focus --app "/^Google/"      # regex: starts with Google');
      Terminal.writeLine('  cwm focus --app "/chrome|safari/i"  # regex: any browser');
      Terminal.writeLine('');
      return;
    }

    // parse --app argument
    const appIndex = args.findIndex(a => a === '--app' || a === '-a');
    if (appIndex === -1 || !args[appIndex + 1]) {
      Terminal.writeLine('Error: --app argument required', 'error');
      return;
    }

    const appName = args[appIndex + 1];
    
    // simulated app list for demo
    const demoApps = ['Safari', 'Google Chrome', 'Terminal', 'Slack', 'Mail', 'Finder', 'VS Code'];
    
    // determine match type for display
    const inputLower = appName.toLowerCase();
    let matchedApp = appName;
    let matchType = '';
    
    // check for regex pattern
    const regexInfo = parseRegexPattern(appName);
    if (regexInfo) {
      try {
        const flags = regexInfo.caseInsensitive ? 'i' : '';
        const regex = new RegExp(regexInfo.pattern, flags);
        const match = demoApps.find(app => regex.test(app));
        if (match) {
          matchedApp = match;
          matchType = `regex: /${regexInfo.pattern}/${regexInfo.caseInsensitive ? 'i' : ''}`;
        } else {
          Terminal.writeLine('');
          Terminal.writeLine(`No app matching regex: ${appName}`, 'error');
          Terminal.writeLine('');
          return;
        }
      } catch (e) {
        Terminal.writeLine('');
        Terminal.writeLine(`Invalid regex pattern: ${appName}`, 'error');
        Terminal.writeLine('');
        return;
      }
    } else {
      // check match type (existing logic)
      const fuzzyMatches = ['safri', 'safarri', 'saffari', 'termnial', 'terminl', 'mai', 'maill', 'slck', 'slak'];
      const titleMatches = ['github', 'taulfsime'];
      
      if (fuzzyMatches.some(f => inputLower.includes(f))) {
        if (inputLower.includes('saf')) matchedApp = 'Safari';
        else if (inputLower.includes('term')) matchedApp = 'Terminal';
        else if (inputLower.includes('slck') || inputLower.includes('slak')) matchedApp = 'Slack';
        else matchedApp = 'Mail';
        matchType = 'fuzzy match';
      } else if (titleMatches.some(t => inputLower.includes(t))) {
        matchedApp = 'Safari';
        matchType = 'title match';
      }
    }
    
    // simulate focus action
    Terminal.writeLine('');
    if (matchType) {
      Terminal.writeLine(`✓ Focused: ${matchedApp} (${matchType})`, 'success');
    } else {
      Terminal.writeLine(`✓ Focused: ${appName}`, 'success');
    }
    Terminal.writeLine('');

    // trigger preview animation
    if (typeof Preview !== 'undefined') {
      Preview.focus(matchedApp.toLowerCase());
    }
  }

  function cmdCwmMaximize(args) {
    if (args.includes('--help') || args.includes('-h')) {
      Terminal.writeLine('');
      Terminal.writeLine('cwm maximize - Maximize a window', 'highlight');
      Terminal.writeLine('');
      Terminal.writeLine('Usage: cwm maximize [--app <name>]');
      Terminal.writeLine('');
      Terminal.writeLine('Options:');
      Terminal.writeLine('  --app, -a <name>  Target app (uses focused if omitted)');
      Terminal.writeLine('  --verbose, -v     Show details');
      Terminal.writeLine('');
      Terminal.writeLine('Examples:');
      Terminal.writeLine('  cwm maximize');
      Terminal.writeLine('  cwm maximize --app Safari');
      Terminal.writeLine('');
      return;
    }

    // parse optional --app argument
    const appIndex = args.findIndex(a => a === '--app' || a === '-a');
    const appName = appIndex !== -1 ? args[appIndex + 1] : null;

    Terminal.writeLine('');
    if (appName) {
      Terminal.writeLine(`✓ Maximized: ${appName}`, 'success');
    } else {
      Terminal.writeLine('✓ Window maximized', 'success');
    }
    Terminal.writeLine('');

    // trigger preview animation
    if (typeof Preview !== 'undefined') {
      Preview.maximize(appName?.toLowerCase());
    }
  }

  function cmdCwmMoveDisplay(args) {
    if (args.includes('--help') || args.includes('-h')) {
      Terminal.writeLine('');
      Terminal.writeLine('cwm move-display - Move window to another display', 'highlight');
      Terminal.writeLine('');
      Terminal.writeLine('Usage: cwm move-display <target> [--app <name>]');
      Terminal.writeLine('');
      Terminal.writeLine('Targets:');
      Terminal.writeLine('  next      Move to next display');
      Terminal.writeLine('  prev      Move to previous display');
      Terminal.writeLine('  <index>   Move to display by index (0-based)');
      Terminal.writeLine('  <alias>   Move to display by alias');
      Terminal.writeLine('');
      Terminal.writeLine('Built-in aliases:');
      Terminal.writeLine('  builtin   Built-in display (MacBook screen)');
      Terminal.writeLine('  external  Any external monitor');
      Terminal.writeLine('  main      Primary display');
      Terminal.writeLine('');
      Terminal.writeLine('Examples:');
      Terminal.writeLine('  cwm move-display next');
      Terminal.writeLine('  cwm move-display external --app Safari');
      Terminal.writeLine('');
      return;
    }

    if (args.length === 0) {
      Terminal.writeLine('Error: display target required', 'error');
      Terminal.writeLine("Usage: cwm move-display <next|prev|index>", 'info');
      return;
    }

    const target = args[0];
    const appIndex = args.findIndex(a => a === '--app' || a === '-a');
    const appName = appIndex !== -1 ? args[appIndex + 1] : null;

    Terminal.writeLine('');
    Terminal.writeLine(`✓ Moved to display: ${target}`, 'success');
    Terminal.writeLine('');

    // trigger preview animation
    if (typeof Preview !== 'undefined') {
      Preview.moveDisplay(target, appName?.toLowerCase());
    }
  }

  function cmdCwmResize(args) {
    if (args.includes('--help') || args.includes('-h')) {
      Terminal.writeLine('');
      Terminal.writeLine('cwm resize - Resize window to percentage', 'highlight');
      Terminal.writeLine('');
      Terminal.writeLine('Usage: cwm resize <size> [--app <name>]');
      Terminal.writeLine('');
      Terminal.writeLine('Size:');
      Terminal.writeLine('  <1-100>   Percentage of screen');
      Terminal.writeLine('  full      100% (same as maximize)');
      Terminal.writeLine('');
      Terminal.writeLine('Options:');
      Terminal.writeLine('  --app, -a <name>  Target app (uses focused if omitted)');
      Terminal.writeLine('');
      Terminal.writeLine('Examples:');
      Terminal.writeLine('  cwm resize 75');
      Terminal.writeLine('  cwm resize full --app Safari');
      Terminal.writeLine('');
      return;
    }

    if (args.length === 0) {
      Terminal.writeLine('Error: size argument required', 'error');
      Terminal.writeLine("Usage: cwm resize <percentage|full>", 'info');
      return;
    }

    const size = args[0];
    const appIndex = args.findIndex(a => a === '--app' || a === '-a');
    const appName = appIndex !== -1 ? args[appIndex + 1] : null;

    Terminal.writeLine('');
    Terminal.writeLine(`✓ Resized to ${size}%`, 'success');
    Terminal.writeLine('');

    // trigger preview animation
    if (typeof Preview !== 'undefined') {
      const percent = size === 'full' ? 100 : parseInt(size, 10);
      Preview.resize(percent, appName?.toLowerCase());
    }
  }

  // utility
  function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  // public API
  return {
    execute
  };
})();
