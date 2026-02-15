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
    // check for pipe/chain commands first
    if ((input.includes('|') || input.includes('&&')) && input.startsWith('cwm')) {
      if (handlePipeCommand(input)) {
        return;
      }
    }
    
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
      ['cwm move --help', 'Move command help'],
      ['cwm resize --help', 'Resize command help'],
      ['cwm events --help', 'Events command help']
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
      ['Move between displays', 'cwm move --display next'],
      ['Resize to percentage', 'cwm resize --to 75'],
      ['Global hotkeys', 'Ctrl+Alt+S to focus Slack'],
      ['App launch rules', 'Auto-maximize Terminal on launch'],
      ['Spotlight integration', 'Search "cwm: Focus Safari"'],
      ['Scripting support', 'cwm list apps --names | fzf'],
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
      case 'move':
        cmdCwmMove(subArgs);
        break;
      case 'resize':
        cmdCwmResize(subArgs);
        break;
      case 'list':
        cmdCwmList(subArgs);
        break;
      case 'get':
        cmdCwmGet(subArgs);
        break;
      case 'events':
        cmdCwmEvents(subArgs);
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
    Terminal.writeLine('  move           Move window to position/display');
    Terminal.writeLine('  resize         Resize window to target size');
    Terminal.writeLine('  list           List apps, displays, or aliases');
    Terminal.writeLine('  get            Get window information');
    Terminal.writeLine('  events         Subscribe to window events');
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
    const isQuiet = args.includes('--quiet') || args.includes('-q');
    
    // simulated app list for demo (matches actual preview windows)
    const demoApps = ['Safari', 'Mail', 'Terminal', 'VS Code'];
    
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
    if (!isQuiet) {
      Terminal.writeLine('');
      if (matchType) {
        Terminal.writeLine(`✓ Focused: ${matchedApp} (${matchType})`, 'success');
      } else {
        Terminal.writeLine(`✓ Focused: ${appName}`, 'success');
      }
      Terminal.writeLine('');
    }

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

  function cmdCwmMove(args) {
    if (args.includes('--help') || args.includes('-h')) {
      Terminal.writeLine('');
      Terminal.writeLine('cwm move - Move window to position and/or display', 'highlight');
      Terminal.writeLine('');
      Terminal.writeLine('Usage: cwm move [--to <pos>] [--display <target>] [--app <name>]');
      Terminal.writeLine('');
      Terminal.writeLine('Position (--to):');
      Terminal.writeLine('  top-left, top, top-right, left, center, right...');
      Terminal.writeLine('  50%,25%        Percentage of screen');
      Terminal.writeLine('  100,200        Absolute coordinates');
      Terminal.writeLine('');
      Terminal.writeLine('Display (--display):');
      Terminal.writeLine('  next           Move to next display');
      Terminal.writeLine('  prev           Move to previous display');
      Terminal.writeLine('  <index>        Move to display by index');
      Terminal.writeLine('  external       Any external monitor');
      Terminal.writeLine('');
      Terminal.writeLine('Examples:');
      Terminal.writeLine('  cwm move --display next');
      Terminal.writeLine('  cwm move --to top-left --display external');
      Terminal.writeLine('  cwm move --to 50%,50% --app Safari');
      Terminal.writeLine('');
      return;
    }

    // parse --display argument
    const displayIndex = args.findIndex(a => a === '--display' || a === '-d');
    const target = displayIndex !== -1 ? args[displayIndex + 1] : null;
    
    // parse --to argument
    const toIndex = args.findIndex(a => a === '--to' || a === '-t');
    const position = toIndex !== -1 ? args[toIndex + 1] : null;
    
    // parse --app argument
    const appIndex = args.findIndex(a => a === '--app' || a === '-a');
    const appName = appIndex !== -1 ? args[appIndex + 1] : null;

    if (!target && !position) {
      Terminal.writeLine('Error: --to or --display required', 'error');
      Terminal.writeLine("Usage: cwm move [--to <pos>] [--display <target>]", 'info');
      return;
    }

    Terminal.writeLine('');
    if (target && position) {
      Terminal.writeLine(`✓ Moved to ${position} on display: ${target}`, 'success');
    } else if (target) {
      Terminal.writeLine(`✓ Moved to display: ${target}`, 'success');
    } else {
      Terminal.writeLine(`✓ Moved to position: ${position}`, 'success');
    }
    Terminal.writeLine('');

    // trigger preview animation (only for display moves)
    if (typeof Preview !== 'undefined' && target) {
      Preview.moveDisplay(target, appName?.toLowerCase());
    }
  }

  function cmdCwmResize(args) {
    if (args.includes('--help') || args.includes('-h')) {
      Terminal.writeLine('');
      Terminal.writeLine('cwm resize - Resize window to target size', 'highlight');
      Terminal.writeLine('');
      Terminal.writeLine('Usage: cwm resize --to <size> [--app <name>]');
      Terminal.writeLine('');
      Terminal.writeLine('Size (--to):');
      Terminal.writeLine('  75, 80%        Percentage of screen');
      Terminal.writeLine('  1920px         Width in pixels');
      Terminal.writeLine('  1920x1080px    Width and height in pixels');
      Terminal.writeLine('  800pt          Width in points');
      Terminal.writeLine('  full           100% (same as maximize)');
      Terminal.writeLine('');
      Terminal.writeLine('Options:');
      Terminal.writeLine('  --app, -a <name>  Target app (uses focused if omitted)');
      Terminal.writeLine('');
      Terminal.writeLine('Examples:');
      Terminal.writeLine('  cwm resize --to 75');
      Terminal.writeLine('  cwm resize --to 1920x1080px --app Safari');
      Terminal.writeLine('');
      return;
    }

    // parse --to argument
    const toIndex = args.findIndex(a => a === '--to' || a === '-t');
    if (toIndex === -1 || !args[toIndex + 1]) {
      Terminal.writeLine('Error: --to argument required', 'error');
      Terminal.writeLine("Usage: cwm resize --to <size>", 'info');
      return;
    }

    const size = args[toIndex + 1];
    const appIndex = args.findIndex(a => a === '--app' || a === '-a');
    const appName = appIndex !== -1 ? args[appIndex + 1] : null;

    Terminal.writeLine('');
    if (size.includes('px') || size.includes('pt')) {
      Terminal.writeLine(`✓ Resized to ${size}`, 'success');
    } else {
      Terminal.writeLine(`✓ Resized to ${size}%`, 'success');
    }
    Terminal.writeLine('');

    // trigger preview animation
    if (typeof Preview !== 'undefined') {
      const percent = size === 'full' ? 100 : parseInt(size, 10);
      Preview.resize(percent, appName?.toLowerCase());
    }
  }

  function cmdCwmList(args) {
    if (args.includes('--help') || args.includes('-h')) {
      Terminal.writeLine('');
      Terminal.writeLine('cwm list - List resources', 'highlight');
      Terminal.writeLine('');
      Terminal.writeLine('Usage: cwm list <resource> [options]');
      Terminal.writeLine('');
      Terminal.writeLine('Resources:');
      Terminal.writeLine('  apps       List running applications');
      Terminal.writeLine('  displays   List available displays');
      Terminal.writeLine('  aliases    List display aliases');
      Terminal.writeLine('  events     List available event types');
      Terminal.writeLine('');
      Terminal.writeLine('Options:');
      Terminal.writeLine('  --json           Output as JSON');
      Terminal.writeLine('  --names          One name per line (for piping)');
      Terminal.writeLine('  --format <fmt>   Custom format string');
      Terminal.writeLine('  --detailed       Include additional fields');
      Terminal.writeLine('');
      Terminal.writeLine('Examples:');
      Terminal.writeLine('  cwm list apps --names');
      Terminal.writeLine('  cwm list apps --names | fzf | xargs cwm focus --app');
      Terminal.writeLine("  cwm list apps --format '{name} ({pid})'");
      Terminal.writeLine('  cwm list displays --json');
      Terminal.writeLine('  cwm list events');
      Terminal.writeLine('');
      return;
    }

    const resource = args[0];
    
    if (!resource) {
      Terminal.writeLine('');
      Terminal.writeLine('Available resources: apps, displays, aliases, events', 'info');
      Terminal.writeLine("Run 'cwm list <resource> --help' for details", 'info');
      Terminal.writeLine('');
      return;
    }

    const hasNames = args.includes('--names');
    const hasJson = args.includes('--json');
    const formatIndex = args.findIndex(a => a === '--format');
    const formatStr = formatIndex !== -1 ? args[formatIndex + 1] : null;

    Terminal.writeLine('');

    if (resource === 'apps') {
      if (hasNames) {
        Terminal.writeLine('Safari');
        Terminal.writeLine('Mail');
        Terminal.writeLine('Terminal');
        Terminal.writeLine('VS Code');
      } else if (hasJson) {
        Terminal.writeLine('{"jsonrpc":"2.0","result":{"items":[', 'output');
        Terminal.writeLine('  {"name":"Safari","pid":1234},', 'output');
        Terminal.writeLine('  {"name":"Mail","pid":5678},', 'output');
        Terminal.writeLine('  {"name":"Terminal","pid":9012},', 'output');
        Terminal.writeLine('  {"name":"VS Code","pid":3456}', 'output');
        Terminal.writeLine(']},"id":null}', 'output');
      } else if (formatStr) {
        // simulate format string output
        Terminal.writeLine('Safari (1234)');
        Terminal.writeLine('Mail (5678)');
        Terminal.writeLine('Terminal (9012)');
        Terminal.writeLine('VS Code (3456)');
      } else {
        Terminal.writeLine('Running Applications:', 'highlight');
        Terminal.writeLine('  Safari      (PID: 1234)');
        Terminal.writeLine('  Mail        (PID: 5678)');
        Terminal.writeLine('  Terminal    (PID: 9012)');
        Terminal.writeLine('  VS Code     (PID: 3456)');
      }
    } else if (resource === 'displays') {
      if (hasNames) {
        Terminal.writeLine('Built-in Retina Display');
        Terminal.writeLine('LG UltraFine');
      } else if (hasJson) {
        Terminal.writeLine('{"jsonrpc":"2.0","result":{"items":[', 'output');
        Terminal.writeLine('  {"index":0,"name":"Built-in Retina Display"},', 'output');
        Terminal.writeLine('  {"index":1,"name":"LG UltraFine"}', 'output');
        Terminal.writeLine(']},"id":null}', 'output');
      } else {
        Terminal.writeLine('Available Displays:', 'highlight');
        Terminal.writeLine('  0: Built-in Retina Display (2560x1600)');
        Terminal.writeLine('  1: LG UltraFine (3840x2160)');
      }
    } else if (resource === 'aliases') {
      Terminal.writeLine('Display Aliases:', 'highlight');
      Terminal.writeLine('  builtin    → Built-in display');
      Terminal.writeLine('  external   → External monitors');
      Terminal.writeLine('  main       → Primary display');
    } else if (resource === 'events') {
      if (hasJson) {
        Terminal.writeLine('{"jsonrpc":"2.0","result":{"events":[', 'output');
        Terminal.writeLine('  "daemon.started","daemon.stopped",', 'output');
        Terminal.writeLine('  "app.launched","app.focused",', 'output');
        Terminal.writeLine('  "window.maximized","window.resized","window.moved"', 'output');
        Terminal.writeLine(']},"id":null}', 'output');
      } else {
        Terminal.writeLine('Available Event Types:', 'highlight');
        Terminal.writeLine('');
        Terminal.writeLine('Daemon events:');
        Terminal.writeLine('  daemon.started   Daemon process started');
        Terminal.writeLine('  daemon.stopped   Daemon process stopped');
        Terminal.writeLine('');
        Terminal.writeLine('App events:');
        Terminal.writeLine('  app.launched     Application launched (via app_rules)');
        Terminal.writeLine('  app.focused      Application window focused');
        Terminal.writeLine('');
        Terminal.writeLine('Window events:');
        Terminal.writeLine('  window.maximized Window maximized');
        Terminal.writeLine('  window.resized   Window resized');
        Terminal.writeLine('  window.moved     Window moved to position/display');
      }
    } else {
      Terminal.writeLine(`Unknown resource: ${resource}`, 'error');
      Terminal.writeLine('Available: apps, displays, aliases, events', 'info');
    }

    Terminal.writeLine('');
  }

  function cmdCwmGet(args) {
    if (args.includes('--help') || args.includes('-h')) {
      Terminal.writeLine('');
      Terminal.writeLine('cwm get - Get window information', 'highlight');
      Terminal.writeLine('');
      Terminal.writeLine('Usage: cwm get <target> [options]');
      Terminal.writeLine('');
      Terminal.writeLine('Targets:');
      Terminal.writeLine('  focused    Get info about focused window');
      Terminal.writeLine('  window     Get info about specific app window');
      Terminal.writeLine('');
      Terminal.writeLine('Options:');
      Terminal.writeLine('  --app <name>     Target app (for window)');
      Terminal.writeLine('  --json           Output as JSON');
      Terminal.writeLine('  --format <fmt>   Custom format string');
      Terminal.writeLine('');
      Terminal.writeLine('Examples:');
      Terminal.writeLine('  cwm get focused');
      Terminal.writeLine('  cwm get focused --json | jq .result.app.name');
      Terminal.writeLine("  cwm get focused --format '{app.name}: {window.width}x{window.height}'");
      Terminal.writeLine('  cwm get window --app Safari');
      Terminal.writeLine('');
      return;
    }

    const target = args[0];
    const hasJson = args.includes('--json');
    const formatIndex = args.findIndex(a => a === '--format');
    const formatStr = formatIndex !== -1 ? args[formatIndex + 1] : null;

    Terminal.writeLine('');

    if (target === 'focused') {
      if (hasJson) {
        Terminal.writeLine('{"jsonrpc":"2.0","result":{', 'output');
        Terminal.writeLine('  "app":{"name":"Safari","pid":1234},', 'output');
        Terminal.writeLine('  "window":{"x":100,"y":50,"width":1200,"height":800}', 'output');
        Terminal.writeLine('},"id":null}', 'output');
      } else if (formatStr) {
        Terminal.writeLine('Safari: 1200x800');
      } else {
        Terminal.writeLine('Focused Window:', 'highlight');
        Terminal.writeLine('  App: Safari (PID: 1234)');
        Terminal.writeLine('  Position: 100, 50');
        Terminal.writeLine('  Size: 1200 x 800');
      }
    } else if (target === 'window') {
      const appIndex = args.findIndex(a => a === '--app' || a === '-a');
      const appName = appIndex !== -1 ? args[appIndex + 1] : 'Safari';
      
      Terminal.writeLine(`Window Info: ${appName}`, 'highlight');
      Terminal.writeLine('  Position: 100, 50');
      Terminal.writeLine('  Size: 1200 x 800');
      Terminal.writeLine('  Display: 0 (Built-in)');
    } else {
      Terminal.writeLine('Error: target required (focused or window)', 'error');
    }

    Terminal.writeLine('');
  }

  function cmdCwmEvents(args) {
    if (args.includes('--help') || args.includes('-h')) {
      Terminal.writeLine('');
      Terminal.writeLine('cwm events - Subscribe to window events', 'highlight');
      Terminal.writeLine('');
      Terminal.writeLine('Usage: cwm events <subcommand> [options]');
      Terminal.writeLine('');
      Terminal.writeLine('Subcommands:');
      Terminal.writeLine('  listen     Stream events to stdout');
      Terminal.writeLine('  wait       Block until specific event occurs');
      Terminal.writeLine('');
      Terminal.writeLine('Listen options:');
      Terminal.writeLine('  --event <pattern>   Filter by event type (glob pattern)');
      Terminal.writeLine('  --app <name>        Filter by app name (supports regex)');
      Terminal.writeLine('');
      Terminal.writeLine('Wait options:');
      Terminal.writeLine('  --event <type>      Event type to wait for (required)');
      Terminal.writeLine('  --app <name>        Filter by app name');
      Terminal.writeLine('  --timeout <secs>    Timeout in seconds (default: no timeout)');
      Terminal.writeLine('  --quiet             No output, exit code only');
      Terminal.writeLine('');
      Terminal.writeLine('Examples:');
      Terminal.writeLine('  cwm events listen');
      Terminal.writeLine('  cwm events listen --event "app.*"');
      Terminal.writeLine('  cwm events listen --app Safari');
      Terminal.writeLine('  cwm events wait --event app.focused --app Safari');
      Terminal.writeLine('  cwm events wait --event window.maximized --timeout 30');
      Terminal.writeLine('');
      return;
    }

    const subcommand = args[0];

    if (!subcommand) {
      Terminal.writeLine('');
      Terminal.writeLine('Subcommands: listen, wait', 'info');
      Terminal.writeLine("Run 'cwm events --help' for details", 'info');
      Terminal.writeLine('');
      return;
    }

    if (subcommand === 'listen') {
      cmdCwmEventsListen(args.slice(1));
    } else if (subcommand === 'wait') {
      cmdCwmEventsWait(args.slice(1));
    } else {
      Terminal.writeLine(`Unknown subcommand: ${subcommand}`, 'error');
      Terminal.writeLine('Available: listen, wait', 'info');
    }
  }

  function cmdCwmEventsListen(args) {
    if (args.includes('--help') || args.includes('-h')) {
      Terminal.writeLine('');
      Terminal.writeLine('cwm events listen - Stream events to stdout', 'highlight');
      Terminal.writeLine('');
      Terminal.writeLine('Usage: cwm events listen [options]');
      Terminal.writeLine('');
      Terminal.writeLine('Options:');
      Terminal.writeLine('  --event <pattern>   Filter by event type (glob: "app.*")');
      Terminal.writeLine('  --app <name>        Filter by app name (repeatable)');
      Terminal.writeLine('');
      Terminal.writeLine('Examples:');
      Terminal.writeLine('  cwm events listen');
      Terminal.writeLine('  cwm events listen --event "window.*"');
      Terminal.writeLine('  cwm events listen --app Safari --app Chrome');
      Terminal.writeLine('  cwm events listen | jq .event');
      Terminal.writeLine('');
      return;
    }

    // parse --event filter
    const eventIndex = args.findIndex(a => a === '--event');
    const eventFilter = eventIndex !== -1 ? args[eventIndex + 1] : null;

    // parse --app filter
    const appIndex = args.findIndex(a => a === '--app');
    const appFilter = appIndex !== -1 ? args[appIndex + 1] : null;

    Terminal.writeLine('');
    Terminal.writeLine('Listening for events... (simulated)', 'info');
    if (eventFilter) {
      Terminal.writeLine(`  Event filter: ${eventFilter}`, 'info');
    }
    if (appFilter) {
      Terminal.writeLine(`  App filter: ${appFilter}`, 'info');
    }
    Terminal.writeLine('');
    
    // simulate some events
    const events = [
      { event: 'app.focused', ts: Date.now(), app: 'Safari', pid: 1234 },
      { event: 'window.resized', ts: Date.now() + 100, app: 'Safari', width: 1200, height: 800 },
      { event: 'window.moved', ts: Date.now() + 200, app: 'Terminal', x: 100, y: 50, display: 0 }
    ];

    events.forEach(evt => {
      // apply filters
      if (eventFilter && !matchGlob(evt.event, eventFilter)) return;
      if (appFilter && evt.app.toLowerCase() !== appFilter.toLowerCase()) return;
      
      Terminal.writeLine(JSON.stringify(evt), 'output');
    });

    Terminal.writeLine('');
    Terminal.writeLine('(In real usage, this streams continuously until Ctrl+C)', 'info');
    Terminal.writeLine('');
  }

  function cmdCwmEventsWait(args) {
    if (args.includes('--help') || args.includes('-h')) {
      Terminal.writeLine('');
      Terminal.writeLine('cwm events wait - Block until specific event', 'highlight');
      Terminal.writeLine('');
      Terminal.writeLine('Usage: cwm events wait --event <type> [options]');
      Terminal.writeLine('');
      Terminal.writeLine('Options:');
      Terminal.writeLine('  --event <type>      Event type to wait for (required)');
      Terminal.writeLine('  --app <name>        Filter by app name');
      Terminal.writeLine('  --timeout <secs>    Timeout in seconds');
      Terminal.writeLine('  --quiet             No output, exit code only');
      Terminal.writeLine('');
      Terminal.writeLine('Exit codes:');
      Terminal.writeLine('  0    Event received');
      Terminal.writeLine('  1    Error');
      Terminal.writeLine('  8    Timeout');
      Terminal.writeLine('  9    Daemon not running');
      Terminal.writeLine('');
      Terminal.writeLine('Examples:');
      Terminal.writeLine('  cwm events wait --event app.focused --app Safari');
      Terminal.writeLine('  cwm events wait --event window.maximized --timeout 10');
      Terminal.writeLine('  cwm events wait --event app.launched --quiet && echo "App launched!"');
      Terminal.writeLine('');
      return;
    }

    // parse --event
    const eventIndex = args.findIndex(a => a === '--event');
    if (eventIndex === -1 || !args[eventIndex + 1]) {
      Terminal.writeLine('Error: --event argument required', 'error');
      Terminal.writeLine("Usage: cwm events wait --event <type>", 'info');
      return;
    }
    const eventType = args[eventIndex + 1];

    // parse --app
    const appIndex = args.findIndex(a => a === '--app');
    const appFilter = appIndex !== -1 ? args[appIndex + 1] : null;

    // parse --timeout
    const timeoutIndex = args.findIndex(a => a === '--timeout');
    const timeout = timeoutIndex !== -1 ? parseInt(args[timeoutIndex + 1], 10) : null;

    const isQuiet = args.includes('--quiet') || args.includes('-q');

    Terminal.writeLine('');
    Terminal.writeLine(`Waiting for event: ${eventType}`, 'info');
    if (appFilter) {
      Terminal.writeLine(`  App filter: ${appFilter}`, 'info');
    }
    if (timeout) {
      Terminal.writeLine(`  Timeout: ${timeout}s`, 'info');
    }
    Terminal.writeLine('');

    // simulate receiving the event
    setTimeout(() => {
      const event = {
        event: eventType,
        ts: Date.now(),
        app: appFilter || 'Safari',
        pid: 1234
      };

      if (!isQuiet) {
        Terminal.writeLine(JSON.stringify(event), 'output');
      }
      Terminal.writeLine('');
      Terminal.writeLine('✓ Event received', 'success');
      Terminal.writeLine('');
    }, 500);
  }

  // simple glob matching for event filters
  function matchGlob(str, pattern) {
    const regex = new RegExp('^' + pattern.replace(/\*/g, '.*').replace(/\?/g, '.') + '$');
    return regex.test(str);
  }

  // simulated pipe command handler
  function handlePipeCommand(input) {
    // detect pipe patterns and simulate output
    if (input.includes('|')) {
      const parts = input.split('|').map(p => p.trim());
      
      // cwm list apps --names | fzf | xargs cwm focus --app
      if (parts[0].includes('list apps --names') && input.includes('fzf')) {
        Terminal.writeLine('');
        Terminal.writeLine('# (simulated fzf selection)', 'info');
        Terminal.writeLine('> Safari', 'output');
        Terminal.writeLine('✓ Focused: Safari', 'success');
        Terminal.writeLine('');
        if (typeof Preview !== 'undefined') {
          Preview.focus('safari');
        }
        return true;
      }
      
      // cwm get focused --json | jq .result.app.name
      if (parts[0].includes('get focused --json') && input.includes('jq')) {
        Terminal.writeLine('');
        Terminal.writeLine('"Safari"', 'output');
        Terminal.writeLine('');
        return true;
      }
    }
    
    // cwm focus --app X && echo/cwm command
    if (input.includes('&&')) {
      const parts = input.split('&&').map(p => p.trim());
      
      if (parts[0].includes('cwm focus')) {
        // extract app name
        const match = parts[0].match(/--app\s+(\S+)/);
        const appName = match ? match[1] : 'Safari';
        
        Terminal.writeLine('');
        Terminal.writeLine(`✓ Focused: ${appName}`, 'success');
        
        if (typeof Preview !== 'undefined') {
          Preview.focus(appName.toLowerCase());
        }
        
        // handle second command
        if (parts[1].includes('echo')) {
          const echoMatch = parts[1].match(/echo\s+"?([^"]+)"?/);
          if (echoMatch) {
            Terminal.writeLine(echoMatch[1], 'output');
          } else {
            Terminal.writeLine('Focused!', 'output');
          }
        } else if (parts[1].includes('cwm maximize')) {
          Terminal.writeLine('✓ Window maximized', 'success');
          if (typeof Preview !== 'undefined') {
            Preview.maximize();
          }
        }
        
        Terminal.writeLine('');
        return true;
      }
    }
    
    return false;
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
