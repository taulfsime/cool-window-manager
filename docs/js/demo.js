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
    { type: 'print', text: 'Watch the preview panel as commands execute...', class: 'info' },
    { type: 'print', text: '' },
    { type: 'wait', duration: 1000 },
    
    // focus by app name
    { type: 'type', text: 'cwm focus --app Mail' },
    { type: 'wait', duration: 800 },
    
    // maximize command
    { type: 'type', text: 'cwm maximize' },
    { type: 'wait', duration: 1000 },
    
    // move display command
    { type: 'type', text: 'cwm move-display next' },
    { type: 'wait', duration: 1200 },
    
    // fuzzy search - "safri" matches "Safari"
    { type: 'print', text: '' },
    { type: 'print', text: '# Fuzzy matching - typos are ok!', class: 'info' },
    { type: 'type', text: 'cwm focus --app safri' },
    { type: 'wait', duration: 1000 },
    
    // resize command
    { type: 'type', text: 'cwm resize 75' },
    { type: 'wait', duration: 1000 },
    
    // match by window title
    { type: 'print', text: '' },
    { type: 'print', text: '# Match by window title', class: 'info' },
    { type: 'type', text: 'cwm focus --app "GitHub - taulfsime"' },
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
    const input = document.getElementById('terminal-input');
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

    // parse and execute
    const parts = text.split(/\s+/);
    if (parts[0] === 'cwm') {
      executeCwmCommand(parts.slice(1));
    }

    await sleep(200);
  }

  // execute cwm command for demo (simplified)
  function executeCwmCommand(args) {
    if (args.length === 0) return;

    const subcommand = args[0];
    const subArgs = args.slice(1);

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
          }
          
          Terminal.writeLine('');
          if (matchType) {
            Terminal.writeLine(`✓ Focused: ${matchedApp} (${matchType})`, 'success');
          } else {
            Terminal.writeLine(`✓ Focused: ${appName}`, 'success');
          }
          Terminal.writeLine('');
          if (typeof Preview !== 'undefined') {
            Preview.focus(appName.toLowerCase());
          }
        }
        break;
      }

      case 'maximize': {
        Terminal.writeLine('');
        Terminal.writeLine('✓ Window maximized', 'success');
        Terminal.writeLine('');
        if (typeof Preview !== 'undefined') {
          Preview.maximize();
        }
        break;
      }

      case 'move-display': {
        const target = subArgs[0] || 'next';
        Terminal.writeLine('');
        Terminal.writeLine(`✓ Moved to display: ${target}`, 'success');
        Terminal.writeLine('');
        if (typeof Preview !== 'undefined') {
          Preview.moveDisplay(target);
        }
        break;
      }

      case 'resize': {
        const size = subArgs[0] || '75';
        Terminal.writeLine('');
        Terminal.writeLine(`✓ Resized to ${size}%`, 'success');
        Terminal.writeLine('');
        if (typeof Preview !== 'undefined') {
          const percent = size === 'full' ? 100 : parseInt(size, 10);
          Preview.resize(percent);
        }
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
