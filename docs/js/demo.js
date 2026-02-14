// auto-demo sequence for cwm landing page

const Demo = (function() {
  'use strict';

  let isRunning = false;
  let escPressedOnce = false;
  let escWarningTimeout = null;

  // demo sequence
  const sequence = [
    { type: 'wait', duration: 800 },
    { type: 'print', text: 'Welcome to cwm - Cool Window Manager for macOS', class: 'highlight' },
    { type: 'print', text: '' },
    { type: 'wait', duration: 600 },
    { type: 'print', text: 'Watch the preview as commands execute...', class: 'info' },
    { type: 'print', text: '' },
    { type: 'wait', duration: 1000 },
    
    // ========== BASIC COMMANDS ==========
    
    // focus by app name
    { type: 'print', text: '# Focus apps by name', class: 'info' },
    { type: 'type', text: 'cwm focus --app Mail' },
    { type: 'wait', duration: 800 },
    
    // maximize command
    { type: 'print', text: '' },
    { type: 'print', text: '# Maximize windows', class: 'info' },
    { type: 'type', text: 'cwm maximize' },
    { type: 'wait', duration: 1000 },
    
    // move display command (skipped on mobile - including the comment)
    { type: 'print', text: '', skipOnMobile: true },
    { type: 'print', text: '# Move between displays', class: 'info', skipOnMobile: true },
    { type: 'type', text: 'cwm move --display next', skipOnMobile: true },
    { type: 'wait', duration: 1200, skipOnMobile: true },
    
    // resize command
    { type: 'print', text: '' },
    { type: 'print', text: '# Resize to any percentage', class: 'info' },
    { type: 'type', text: 'cwm resize --to 75' },
    { type: 'wait', duration: 1000 },
    
    // ========== SMART MATCHING ==========
    
    // fuzzy search - "safri" matches "Safari"
    { type: 'print', text: '' },
    { type: 'print', text: '# Fuzzy matching - typos are ok!', class: 'info' },
    { type: 'type', text: 'cwm focus --app safri' },
    { type: 'wait', duration: 1000 },
    
    // regex matching
    { type: 'print', text: '' },
    { type: 'print', text: '# Regex patterns for power users', class: 'info' },
    { type: 'type', text: 'cwm focus --app "/^VS/"' },
    { type: 'wait', duration: 1000 },
    
    // case insensitive regex
    { type: 'print', text: '' },
    { type: 'print', text: '# Case-insensitive regex', class: 'info' },
    { type: 'type', text: 'cwm focus --app "/mail/i"' },
    { type: 'wait', duration: 1000 },
    
    // multiple apps fallback
    { type: 'print', text: '' },
    { type: 'print', text: '# Try multiple apps in order', class: 'info' },
    { type: 'type', text: 'cwm focus --app Chrome --app Safari' },
    { type: 'wait', duration: 1000 },
    
    // match by window title
    { type: 'print', text: '' },
    { type: 'print', text: '# Match by window title', class: 'info' },
    { type: 'type', text: 'cwm focus --app "GitHub - taulfsime"' },
    { type: 'wait', duration: 1000 },
    
    // ========== SCRIPTING & AUTOMATION ==========
    
    { type: 'print', text: '' },
    { type: 'print', text: '═══ Scripting & Automation ═══', class: 'highlight' },
    { type: 'print', text: '' },
    { type: 'wait', duration: 800 },
    
    // list apps with --names for piping
    { type: 'print', text: '# List apps (one per line for piping)', class: 'info' },
    { type: 'type', text: 'cwm list apps --names' },
    { type: 'wait', duration: 1000 },
    
    // fzf integration
    { type: 'print', text: '' },
    { type: 'print', text: '# Pipe to fzf for fuzzy selection', class: 'info' },
    { type: 'type', text: 'cwm list apps --names | fzf | xargs cwm focus --app' },
    { type: 'wait', duration: 1200 },
    
    // JSON output for automation
    { type: 'print', text: '' },
    { type: 'print', text: '# JSON output + jq for automation', class: 'info' },
    { type: 'type', text: 'cwm get focused --json | jq .result.app.name' },
    { type: 'wait', duration: 1000 },
    
    // custom format strings
    { type: 'print', text: '' },
    { type: 'print', text: '# Custom format strings', class: 'info' },
    { type: 'type', text: "cwm list apps --format '{name} ({pid})'" },
    { type: 'wait', duration: 1000 },
    
    // chain commands
    { type: 'print', text: '' },
    { type: 'print', text: '# Chain commands with &&', class: 'info' },
    { type: 'type', text: 'cwm focus --app Terminal && cwm maximize' },
    { type: 'wait', duration: 1000 },
    
    // exit codes
    { type: 'print', text: '' },
    { type: 'print', text: '# Exit codes for error handling', class: 'info' },
    { type: 'type', text: 'cwm focus --app Safari && echo "Focused!"' },
    { type: 'wait', duration: 1000 },
    
    // quiet mode
    { type: 'print', text: '' },
    { type: 'print', text: '# Quiet mode for scripts', class: 'info' },
    { type: 'type', text: 'cwm focus --app Mail --quiet' },
    { type: 'wait', duration: 800 },
    
    // get window info
    { type: 'print', text: '' },
    { type: 'print', text: '# Get window information', class: 'info' },
    { type: 'type', text: 'cwm get focused' },
    { type: 'wait', duration: 1000 },
    
    // list displays
    { type: 'print', text: '' },
    { type: 'print', text: '# List available displays', class: 'info' },
    { type: 'type', text: 'cwm list displays' },
    { type: 'wait', duration: 1000 },
    
    // ========== MORE EXAMPLES ==========
    
    // resize with different units
    { type: 'print', text: '' },
    { type: 'print', text: '# Resize with pixels', class: 'info' },
    { type: 'type', text: 'cwm resize --to 1200px --app Safari' },
    { type: 'wait', duration: 1000 },
    
    // another resize
    { type: 'print', text: '' },
    { type: 'print', text: '# Resize to 60%', class: 'info' },
    { type: 'type', text: 'cwm resize --to 60' },
    { type: 'wait', duration: 800 },
    
    // final message
    { type: 'print', text: '' },
    { type: 'print', text: "Type 'help' to explore more commands", class: 'info' },
    { type: 'print', text: "Type 'install' to get started", class: 'info' },
    { type: 'print', text: '' }
  ];

  // run the demo
  async function run() {
    if (isRunning) {
      Terminal.writeLine('Demo already running...', 'info');
      return;
    }

    isRunning = true;
    Terminal.lock();

    // reset preview
    if (typeof Preview !== 'undefined') {
      Preview.reset();
    }

    try {
      for (const step of sequence) {
        if (!isRunning) break;

        // skip steps marked for mobile skip
        if (step.skipOnMobile && isMobile()) {
          continue;
        }

        switch (step.type) {
          case 'wait':
            await sleep(step.duration);
            break;

          case 'print':
            Terminal.writeLine(step.text, step.class || 'output');
            break;

          case 'type':
            await typeCommand(step.text);
            break;
        }
      }
    } catch (e) {
      console.error('Demo error:', e);
    }

    isRunning = false;
    Terminal.unlock();
  }

  // type a command with animation
  async function typeCommand(text) {
    // get the appropriate input element
    const input = isMobile() 
      ? document.getElementById('mobile-input')
      : document.getElementById('terminal-input');
    if (!input) return;

    input.value = '';
    
    // type each character
    for (const char of text) {
      if (!isRunning) break;
      input.value += char;
      await sleep(40 + Math.random() * 40);
    }

    await sleep(300);

    // execute the command
    Terminal.writeCommand(text);
    input.value = '';

    // check for pipe/chain commands
    if (text.includes('|') || text.includes('&&')) {
      // use Commands handler for pipe commands
      if (typeof Commands !== 'undefined') {
        Commands.execute(text);
      }
    } else {
      // parse and execute cwm commands
      const parts = text.split(/\s+/);
      if (parts[0] === 'cwm') {
        executeCwmCommand(parts.slice(1));
      }
    }

    await sleep(200);
  }

  // check if on mobile
  function isMobile() {
    return window.matchMedia('(max-width: 640px)').matches;
  }

  // execute cwm command for demo (simplified)
  function executeCwmCommand(args) {
    if (args.length === 0) return;

    const subcommand = args[0];
    const subArgs = args.slice(1);

    // check for --quiet flag
    const isQuiet = subArgs.includes('--quiet') || subArgs.includes('-q');

    switch (subcommand) {
      case 'focus': {
        const appIndex = subArgs.findIndex(a => a === '--app' || a === '-a');
        const appName = appIndex !== -1 ? subArgs[appIndex + 1] : null;
        if (appName) {
          const inputLower = appName.toLowerCase();
          let matchedApp = appName;
          let matchType = '';
          
          // detect match type for demo output
          if (inputLower === 'safri' || inputLower === 'safarri') {
            matchedApp = 'Safari';
            matchType = 'fuzzy match';
          } else if (inputLower.includes('github') || inputLower.includes('taulfsime')) {
            matchedApp = 'Safari';
            matchType = 'title match';
          } else if (inputLower.startsWith('/')) {
            // regex pattern
            if (inputLower.includes('vs')) {
              matchedApp = 'VS Code';
              matchType = 'regex: /^VS/';
            } else if (inputLower.includes('mail')) {
              matchedApp = 'Mail';
              matchType = 'regex: /mail/i';
            }
          } else if (inputLower === 'chrome') {
            // fallback demo - Chrome not found, try Safari
            matchedApp = 'Safari';
            matchType = 'fallback';
          }
          
          if (!isQuiet) {
            Terminal.writeLine('');
            if (matchType === 'fallback') {
              Terminal.writeLine('Chrome not running, trying next...', 'info');
              Terminal.writeLine(`✓ Focused: ${matchedApp}`, 'success');
            } else if (matchType) {
              Terminal.writeLine(`✓ Focused: ${matchedApp} (${matchType})`, 'success');
            } else {
              Terminal.writeLine(`✓ Focused: ${appName}`, 'success');
            }
            Terminal.writeLine('');
          }
          
          if (typeof Preview !== 'undefined') {
            Preview.focus(matchedApp.toLowerCase().replace(' ', ''));
          }
        }
        break;
      }

      case 'maximize': {
        if (!isQuiet) {
          Terminal.writeLine('');
          Terminal.writeLine('✓ Window maximized', 'success');
          Terminal.writeLine('');
        }
        if (typeof Preview !== 'undefined') {
          Preview.maximize();
        }
        break;
      }

      case 'move': {
        // parse --display argument
        const displayIndex = subArgs.findIndex(a => a === '--display' || a === '-d');
        const target = displayIndex !== -1 ? subArgs[displayIndex + 1] : 'next';
        
        if (!isQuiet) {
          Terminal.writeLine('');
          Terminal.writeLine(`✓ Moved to display: ${target}`, 'success');
          Terminal.writeLine('');
        }
        
        // on mobile, skip actual display move
        if (!isMobile() && typeof Preview !== 'undefined') {
          Preview.moveDisplay(target);
        }
        break;
      }

      case 'resize': {
        // parse --to argument
        const toIndex = subArgs.findIndex(a => a === '--to' || a === '-t');
        const size = toIndex !== -1 ? subArgs[toIndex + 1] : '75';
        
        // parse --app argument
        const appIndex = subArgs.findIndex(a => a === '--app' || a === '-a');
        const appName = appIndex !== -1 ? subArgs[appIndex + 1] : null;
        
        if (!isQuiet) {
          Terminal.writeLine('');
          if (size.includes('px')) {
            Terminal.writeLine(`✓ Resized to ${size}`, 'success');
          } else {
            Terminal.writeLine(`✓ Resized to ${size}%`, 'success');
          }
          Terminal.writeLine('');
        }
        
        if (typeof Preview !== 'undefined') {
          const percent = size === 'full' ? 100 : parseInt(size, 10);
          Preview.resize(percent, appName?.toLowerCase());
        }
        break;
      }

      case 'list': {
        const resource = subArgs[0];
        Terminal.writeLine('');
        
        if (resource === 'apps') {
          if (subArgs.includes('--names')) {
            Terminal.writeLine('Safari');
            Terminal.writeLine('Mail');
            Terminal.writeLine('Terminal');
            Terminal.writeLine('VS Code');
          } else if (subArgs.includes('--format')) {
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
          Terminal.writeLine('Available Displays:', 'highlight');
          Terminal.writeLine('  0: Built-in Retina Display (2560x1600)');
          Terminal.writeLine('  1: LG UltraFine (3840x2160)');
        }
        
        Terminal.writeLine('');
        break;
      }

      case 'get': {
        const target = subArgs[0];
        Terminal.writeLine('');
        
        if (target === 'focused') {
          if (subArgs.includes('--json')) {
            // handled by pipe command handler
          } else {
            Terminal.writeLine('Focused Window:', 'highlight');
            Terminal.writeLine('  App: Safari (PID: 1234)');
            Terminal.writeLine('  Position: 100, 50');
            Terminal.writeLine('  Size: 1200 x 800');
            Terminal.writeLine('  Display: 0 (Built-in)');
          }
        }
        
        Terminal.writeLine('');
        break;
      }
    }
  }

  // stop the demo
  function stop() {
    isRunning = false;
    escPressedOnce = false;
    if (escWarningTimeout) {
      clearTimeout(escWarningTimeout);
      escWarningTimeout = null;
    }
  }

  // handle Ctrl+C to cancel
  function handleCancel() {
    if (!isRunning) return false;

    if (escPressedOnce) {
      // second press - cancel demo
      stop();
      Terminal.writeLine('');
      Terminal.writeLine('^C', 'error');
      Terminal.writeLine('Demo cancelled.', 'info');
      Terminal.writeLine('');
      Terminal.unlock();
      return true;
    } else {
      // first press - show warning
      escPressedOnce = true;
      Terminal.writeLine('');
      Terminal.writeLine('Press Ctrl+C again to skip demo...', 'info');
      
      // reset after 2 seconds
      escWarningTimeout = setTimeout(() => {
        escPressedOnce = false;
      }, 2000);
      
      return true;
    }
  }

  // utility
  function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  // auto-start demo on page load
  function autoStart() {
    // small delay to ensure everything is loaded
    setTimeout(() => {
      run();
    }, 500);
  }

  // initialize on DOM ready
  document.addEventListener('DOMContentLoaded', autoStart);

  // listen for Ctrl+C to cancel demo
  document.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.key === 'c') {
      if (handleCancel()) {
        e.preventDefault();
      }
    }
  });

  // public API
  return {
    run,
    stop,
    isRunning: () => isRunning
  };
})();
