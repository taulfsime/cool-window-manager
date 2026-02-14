// terminal emulator core

const Terminal = (function() {
  'use strict';

  // dom elements
  let outputEl = null;
  let inputEl = null;
  let terminalEl = null;
  let mobileInputEl = null;
  let embeddedOutputEl = null;

  // state
  let history = [];
  let historyIndex = -1;
  let isLocked = false;
  let soundEnabled = false;

  // check if on mobile
  function isMobile() {
    return window.matchMedia('(max-width: 640px)').matches;
  }

  // get the current output element (embedded on mobile, regular on desktop)
  function getOutputEl() {
    if (isMobile() && embeddedOutputEl) {
      return embeddedOutputEl;
    }
    return outputEl;
  }

  // get the current input element (mobile input on mobile, regular on desktop)
  function getInputEl() {
    if (isMobile() && mobileInputEl) {
      return mobileInputEl;
    }
    return inputEl;
  }

  // sound effects (base64 encoded short clicks)
  const sounds = {
    keypress: null,
    enter: null
  };

  // initialize audio context lazily
  function initSounds() {
    if (sounds.keypress) return;
    
    try {
      const AudioContext = window.AudioContext || window.webkitAudioContext;
      const ctx = new AudioContext();
      
      // create simple click sound
      sounds.keypress = () => {
        if (!soundEnabled) return;
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.frequency.value = 800;
        gain.gain.value = 0.05;
        gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.05);
        osc.start(ctx.currentTime);
        osc.stop(ctx.currentTime + 0.05);
      };
      
      sounds.enter = () => {
        if (!soundEnabled) return;
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.frequency.value = 600;
        gain.gain.value = 0.08;
        gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.1);
        osc.start(ctx.currentTime);
        osc.stop(ctx.currentTime + 0.1);
      };
    } catch (e) {
      // audio not supported
      sounds.keypress = () => {};
      sounds.enter = () => {};
    }
  }

  // initialize terminal
  function init() {
    outputEl = document.getElementById('terminal-output');
    inputEl = document.getElementById('terminal-input');
    terminalEl = document.getElementById('terminal');
    mobileInputEl = document.getElementById('mobile-input');
    embeddedOutputEl = document.getElementById('embedded-terminal-output');

    if (!outputEl || !inputEl || !terminalEl) {
      console.error('Terminal elements not found');
      return;
    }

    // event listeners for desktop input
    inputEl.addEventListener('keydown', handleKeyDown);
    inputEl.addEventListener('input', handleInput);
    terminalEl.addEventListener('click', focusInput);

    // event listeners for mobile input
    if (mobileInputEl) {
      mobileInputEl.addEventListener('keydown', handleMobileKeyDown);
      
      const mobileSubmit = document.getElementById('mobile-submit');
      if (mobileSubmit) {
        mobileSubmit.addEventListener('click', handleMobileSubmit);
      }
    }

    // sound toggle
    const soundToggle = document.getElementById('sound-toggle');
    if (soundToggle) {
      soundToggle.addEventListener('click', toggleSound);
    }

    // update menubar time
    updateMenubarTime();
    setInterval(updateMenubarTime, 60000);

    // focus input
    focusInput();
  }

  function handleMobileKeyDown(e) {
    if (isLocked) {
      e.preventDefault();
      return;
    }

    if (e.key === 'Enter') {
      e.preventDefault();
      handleMobileSubmit();
    }
  }

  function handleMobileSubmit() {
    if (!mobileInputEl) return;
    
    const value = mobileInputEl.value.trim();
    if (value) {
      executeCommand(value);
      mobileInputEl.value = '';
    }
  }

  function handleKeyDown(e) {
    if (isLocked) {
      e.preventDefault();
      return;
    }

    initSounds();

    switch (e.key) {
      case 'Enter':
        e.preventDefault();
        sounds.enter?.();
        executeCommand(inputEl.value.trim());
        break;

      case 'ArrowUp':
        e.preventDefault();
        navigateHistory(-1);
        break;

      case 'ArrowDown':
        e.preventDefault();
        navigateHistory(1);
        break;

      case 'Tab':
        e.preventDefault();
        autocomplete();
        break;

      case 'c':
        if (e.ctrlKey) {
          e.preventDefault();
          cancelInput();
        } else {
          sounds.keypress?.();
        }
        break;

      case 'l':
        if (e.ctrlKey) {
          e.preventDefault();
          clear();
        } else {
          sounds.keypress?.();
        }
        break;

      default:
        if (e.key.length === 1 && !e.ctrlKey && !e.metaKey) {
          sounds.keypress?.();
        }
        break;
    }
  }

  function handleInput() {
    // allow free cursor movement
  }

  function focusInput() {
    const target = getInputEl();
    if (target) target.focus();
  }

  function navigateHistory(direction) {
    if (history.length === 0) return;

    historyIndex += direction;

    if (historyIndex < 0) {
      historyIndex = 0;
    } else if (historyIndex >= history.length) {
      historyIndex = history.length;
      inputEl.value = '';
      return;
    }

    inputEl.value = history[historyIndex];
  }

  function autocomplete() {
    const currentInput = getInputEl();
    const input = currentInput.value.trim();
    if (!input) return;

    // simple autocomplete for cwm commands
    const commands = ['help', 'install', 'demo', 'features', 'why', 'github', 'clear', 'cwm'];
    const cwmSubcommands = ['focus', 'maximize', 'move', 'resize', 'list', 'get', '--help'];
    const cwmFlags = ['--app', '--to', '--display', '--json', '--names', '--format', '--help'];

    const parts = input.split(/\s+/);
    const lastPart = parts[parts.length - 1].toLowerCase();

    let matches = [];

    if (parts.length === 1) {
      matches = commands.filter(c => c.startsWith(lastPart));
    } else if (parts[0] === 'cwm') {
      if (parts.length === 2) {
        matches = cwmSubcommands.filter(c => c.startsWith(lastPart));
      } else if (lastPart.startsWith('-')) {
        matches = cwmFlags.filter(c => c.startsWith(lastPart));
      }
    }

    if (matches.length === 1) {
      parts[parts.length - 1] = matches[0];
      currentInput.value = parts.join(' ');
    } else if (matches.length > 1) {
      // show possible completions
      writeLine(`  ${matches.join('  ')}`, 'info');
    }
  }

  function cancelInput() {
    if (inputEl.value) {
      writeLine(`$ ${inputEl.value}^C`, 'command');
      inputEl.value = '';
    }
  }

  function executeCommand(input) {
    if (!input) {
      writeLine('$', 'command');
      return;
    }

    // add to history
    if (history[history.length - 1] !== input) {
      history.push(input);
    }
    historyIndex = history.length;

    // display command
    writeCommand(input);

    // clear input
    inputEl.value = '';

    // execute via Commands module
    if (typeof Commands !== 'undefined') {
      Commands.execute(input);
    } else {
      writeLine('Command system not loaded', 'error');
    }
  }

  // output methods
  function writeCommand(cmd) {
    const target = getOutputEl();
    if (!target) return;
    
    const line = document.createElement('div');
    line.className = 'terminal-line command';
    line.innerHTML = formatCommand(cmd);
    target.appendChild(line);
    scrollToBottom();
  }

  function formatCommand(cmd) {
    // highlight command syntax
    const parts = cmd.split(/\s+/);
    let html = '<span class="prompt">$</span> ';

    parts.forEach((part, i) => {
      if (i === 0) {
        html += `<span class="cmd">${escapeHtml(part)}</span>`;
      } else if (part.startsWith('--') || part.startsWith('-')) {
        html += ` <span class="flag">${escapeHtml(part)}</span>`;
      } else {
        html += ` <span class="value">${escapeHtml(part)}</span>`;
      }
    });

    return html;
  }

  function writeLine(text, className = 'output') {
    const target = getOutputEl();
    if (!target) return;
    
    const line = document.createElement('div');
    line.className = `terminal-line ${className}`;
    line.textContent = text;
    target.appendChild(line);
    scrollToBottom();
  }

  function writeHtml(html, className = 'output') {
    const target = getOutputEl();
    if (!target) return;
    
    const line = document.createElement('div');
    line.className = `terminal-line ${className}`;
    line.innerHTML = html;
    target.appendChild(line);
    scrollToBottom();
  }

  function writeLines(lines) {
    lines.forEach(([text, className]) => {
      writeLine(text, className || 'output');
    });
  }

  function clear() {
    if (outputEl) outputEl.innerHTML = '';
    if (embeddedOutputEl) embeddedOutputEl.innerHTML = '';
  }

  function scrollToBottom() {
    // scroll output container to bottom
    const target = getOutputEl();
    if (!target) return;
    
    requestAnimationFrame(() => {
      target.scrollTop = target.scrollHeight;
    });
  }

  // lock/unlock for demo mode
  function lock() {
    isLocked = true;
    if (inputEl) inputEl.disabled = true;
    if (mobileInputEl) mobileInputEl.disabled = true;
  }

  function unlock() {
    isLocked = false;
    if (inputEl) inputEl.disabled = false;
    if (mobileInputEl) mobileInputEl.disabled = false;
    focusInput();
  }

  // type text character by character (for demo)
  async function typeText(text, delay = 50) {
    lock();
    inputEl.value = '';
    
    for (const char of text) {
      inputEl.value += char;
      sounds.keypress?.();
      await sleep(delay + Math.random() * 30);
    }
    
    await sleep(200);
    sounds.enter?.();
    executeCommand(text);
    unlock();
  }

  // utility
  function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  function toggleSound() {
    initSounds();
    soundEnabled = !soundEnabled;
    
    const toggle = document.getElementById('sound-toggle');
    const onIcon = toggle.querySelector('.sound-on');
    const offIcon = toggle.querySelector('.sound-off');
    
    if (soundEnabled) {
      toggle.classList.add('active');
      onIcon.style.display = 'block';
      offIcon.style.display = 'none';
    } else {
      toggle.classList.remove('active');
      onIcon.style.display = 'none';
      offIcon.style.display = 'block';
    }
  }

  function updateMenubarTime() {
    const timeEl = document.getElementById('menubar-time');
    if (timeEl) {
      const now = new Date();
      const hours = now.getHours();
      const minutes = now.getMinutes().toString().padStart(2, '0');
      const ampm = hours >= 12 ? 'PM' : 'AM';
      const displayHours = hours % 12 || 12;
      timeEl.textContent = `${displayHours}:${minutes} ${ampm}`;
    }
  }

  // public API
  return {
    init,
    writeLine,
    writeHtml,
    writeLines,
    writeCommand,
    clear,
    lock,
    unlock,
    typeText,
    focusInput,
    scrollToBottom
  };
})();

// initialize on DOM ready
document.addEventListener('DOMContentLoaded', Terminal.init);
