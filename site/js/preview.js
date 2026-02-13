// mock desktop preview with window animations

const Preview = (function() {
  'use strict';

  // constants
  const MAX_DISPLAYS = 4;

  // window state
  let windows = {};
  let focusedApp = null;
  let zIndexCounter = 10;
  let displayCount = 2;

  // initial window positions (percentages)
  const initialPositions = {
    safari: { top: 8, left: 5, width: 50, height: 45, display: 1 },
    mail: { top: 15, left: 20, width: 45, height: 40, display: 1 },
    vscode: { top: 25, left: 35, width: 50, height: 50, display: 1 },
    terminal: { top: 50, left: 45, width: 45, height: 35, display: 1 }
  };

  // initialize preview
  function init() {
    const mockWindows = document.querySelectorAll('.mock-window');
    
    mockWindows.forEach(win => {
      const app = win.dataset.app;
      if (app && initialPositions[app]) {
        windows[app] = {
          element: win,
          ...initialPositions[app],
          maximized: false
        };
        applyPosition(app);
      }
    });

    // set initial focus
    focus('vscode');

    // set initial display count
    const displaysContainer = document.getElementById('mock-displays');
    if (displaysContainer) {
      displaysContainer.dataset.count = displayCount;
    }

    // setup add display button
    const addBtn = document.getElementById('add-display-btn');
    if (addBtn) {
      addBtn.addEventListener('click', addDisplay);
      updateAddButton();
    }
  }

  // add a new display
  function addDisplay() {
    if (displayCount >= MAX_DISPLAYS) return;

    displayCount++;
    const displaysContainer = document.getElementById('mock-displays');
    
    const newDisplay = document.createElement('div');
    newDisplay.className = 'mock-display';
    newDisplay.dataset.display = displayCount;
    newDisplay.innerHTML = `<div class="display-label">Display ${displayCount}</div>`;
    
    displaysContainer.appendChild(newDisplay);
    displaysContainer.dataset.count = displayCount;
    updateAddButton();
  }

  // update add button state
  function updateAddButton() {
    const addBtn = document.getElementById('add-display-btn');
    if (addBtn) {
      addBtn.disabled = displayCount >= MAX_DISPLAYS;
      addBtn.textContent = displayCount >= MAX_DISPLAYS 
        ? `Max ${MAX_DISPLAYS} Displays` 
        : '+ Add Display';
    }
  }

  // get current display count
  function getDisplayCount() {
    return displayCount;
  }

  // apply position to window element
  function applyPosition(app) {
    const win = windows[app];
    if (!win) return;

    const el = win.element;
    const display = document.querySelector(`.mock-display[data-display="${win.display}"]`);
    
    if (!display) return;

    // move window to correct display
    if (el.parentElement !== display) {
      display.appendChild(el);
    }

    // reset maximized class
    el.classList.remove('maximized');

    // apply position as percentages
    el.style.top = `${win.top}%`;
    el.style.left = `${win.left}%`;
    el.style.width = `${win.width}%`;
    el.style.height = `${win.height}%`;
    el.style.right = 'auto';
    el.style.bottom = 'auto';
  }

  // focus a window
  function focus(app) {
    // normalize app name
    app = normalizeAppName(app);
    
    const win = windows[app];
    if (!win) {
      // try to find closest match
      const match = findClosestApp(app);
      if (match) {
        app = match;
      } else {
        return;
      }
    }

    // remove focus from all windows
    Object.values(windows).forEach(w => {
      w.element.classList.remove('focused', 'focusing');
    });

    // focus the target window
    const targetWin = windows[app];
    if (targetWin) {
      zIndexCounter++;
      targetWin.element.style.zIndex = zIndexCounter;
      targetWin.element.classList.add('focused', 'focusing');
      focusedApp = app;

      // update menubar
      updateMenubar(app);

      // remove focusing animation class after animation completes
      setTimeout(() => {
        targetWin.element.classList.remove('focusing');
      }, 600);
    }
  }

  // maximize a window
  function maximize(app) {
    // use focused app if not specified
    if (!app) {
      app = focusedApp;
    } else {
      app = normalizeAppName(app);
      const match = findClosestApp(app);
      if (match) app = match;
    }

    const win = windows[app];
    if (!win) return;

    // focus first
    focus(app);

    // toggle maximize
    win.maximized = true;
    win.element.classList.add('maximized');
  }

  // move window to another display
  function moveDisplay(target, app) {
    // use focused app if not specified
    if (!app) {
      app = focusedApp;
    } else {
      app = normalizeAppName(app);
      const match = findClosestApp(app);
      if (match) app = match;
    }

    const win = windows[app];
    if (!win) return;

    // focus first
    focus(app);

    // determine target display
    let targetDisplay = win.display;
    
    if (target === 'next') {
      targetDisplay = win.display >= displayCount ? 1 : win.display + 1;
    } else if (target === 'prev') {
      targetDisplay = win.display <= 1 ? displayCount : win.display - 1;
    } else if (target === 'external') {
      targetDisplay = win.display === 1 ? 2 : 1;
    } else if (!isNaN(parseInt(target))) {
      const num = parseInt(target);
      targetDisplay = Math.max(1, Math.min(num, displayCount));
    } else {
      // default to next
      targetDisplay = win.display >= displayCount ? 1 : win.display + 1;
    }

    // animate the move
    const el = win.element;
    const currentDisplay = document.querySelector(`.mock-display[data-display="${win.display}"]`);
    const newDisplay = document.querySelector(`.mock-display[data-display="${targetDisplay}"]`);

    if (!currentDisplay || !newDisplay || win.display === targetDisplay) return;

    // get current position
    const currentRect = el.getBoundingClientRect();
    const newDisplayRect = newDisplay.getBoundingClientRect();

    // calculate slide direction
    const slideDirection = targetDisplay > win.display ? 1 : -1;

    // add transition for smooth animation
    el.style.transition = 'transform 0.4s ease-out';
    
    // slide out
    el.style.transform = `translateX(${slideDirection * 120}%)`;

    setTimeout(() => {
      // move to new display
      el.style.transition = 'none';
      el.style.transform = 'translateX(0)';
      win.display = targetDisplay;
      win.maximized = false;
      el.classList.remove('maximized');
      
      // move element to new display
      newDisplay.appendChild(el);
      
      // reset position
      applyPosition(app);
      
      // re-enable transition
      setTimeout(() => {
        el.style.transition = '';
      }, 50);
    }, 400);
  }

  // resize window to percentage
  function resize(percent, app) {
    // use focused app if not specified
    if (!app) {
      app = focusedApp;
    } else {
      app = normalizeAppName(app);
      const match = findClosestApp(app);
      if (match) app = match;
    }

    const win = windows[app];
    if (!win) return;

    // focus first
    focus(app);

    // remove maximized state
    win.maximized = false;
    win.element.classList.remove('maximized');

    // calculate new size (centered)
    const newWidth = percent * 0.9; // scale down for preview
    const newHeight = percent * 0.7;
    const newLeft = (100 - newWidth) / 2;
    const newTop = 10 + (80 - newHeight) / 2;

    // update state
    win.width = newWidth;
    win.height = newHeight;
    win.left = newLeft;
    win.top = newTop;

    // apply with animation
    win.element.style.transition = 'all 0.3s ease-out';
    applyPosition(app);
    
    setTimeout(() => {
      win.element.style.transition = '';
    }, 300);
  }

  // reset preview to initial state
  function reset() {
    Object.keys(windows).forEach(app => {
      const win = windows[app];
      win.maximized = false;
      win.display = initialPositions[app].display;
      Object.assign(win, initialPositions[app]);
      win.element.classList.remove('maximized', 'focused', 'focusing');
      applyPosition(app);
    });
    
    zIndexCounter = 10;
    focusedApp = null;
    
    // reset focus to vscode
    setTimeout(() => focus('vscode'), 100);
  }

  // helper: normalize app name for matching
  function normalizeAppName(name) {
    if (!name) return null;
    return name.toLowerCase().trim();
  }

  // helper: find closest matching app
  function findClosestApp(input) {
    if (!input) return null;
    
    input = input.toLowerCase();
    
    // exact match by app name
    if (windows[input]) return input;
    
    // prefix match by app name
    for (const app of Object.keys(windows)) {
      if (app.startsWith(input)) return app;
    }
    
    // match by window title
    for (const app of Object.keys(windows)) {
      const el = windows[app].element;
      const title = el.dataset.title?.toLowerCase() || '';
      if (title && (title.includes(input) || input.includes(title.split(' ')[0]))) {
        return app;
      }
    }
    
    // fuzzy match (simple contains)
    for (const app of Object.keys(windows)) {
      if (app.includes(input) || input.includes(app)) return app;
    }
    
    // common aliases and fuzzy matches
    const aliases = {
      'chrome': 'safari',
      'browser': 'safari',
      'web': 'safari',
      'safri': 'safari',
      'safarri': 'safari',
      'saffari': 'safari',
      'term': 'terminal',
      'iterm': 'terminal',
      'console': 'terminal',
      'termnial': 'terminal',
      'terminl': 'terminal',
      'email': 'mail',
      'outlook': 'mail',
      'mai': 'mail',
      'maill': 'mail'
    };
    
    if (aliases[input]) return aliases[input];
    
    return null;
  }

  // helper: update menubar app name
  function updateMenubar(app) {
    const menubarApp = document.querySelector('.menubar-app');
    if (menubarApp) {
      const displayName = app.charAt(0).toUpperCase() + app.slice(1);
      menubarApp.textContent = displayName;
    }
  }

  // initialize on DOM ready
  document.addEventListener('DOMContentLoaded', init);

  // public API
  return {
    focus,
    maximize,
    moveDisplay,
    resize,
    reset,
    addDisplay,
    getDisplayCount
  };
})();
