# cwm Landing Page - Implementation Guide

This document contains post-implementation steps to deploy the landing page.

## Overview

The landing page is an interactive terminal-style website that showcases cwm features. It includes:

- **Interactive terminal**: Users can type commands and see simulated output
- **Desktop preview**: Animated mock macOS desktop showing window manipulations
- **Auto-demo**: On page load, demonstrates focus → maximize → move-display → resize
- **Red theme**: Custom dark theme with red accents

## Files Created

```
site/
├── index.html          # Main HTML structure
├── CNAME               # Custom domain: cwm.taulfsime.com
├── css/
│   └── style.css       # Red theme, terminal styling, animations (~450 lines)
├── js/
│   ├── terminal.js     # Terminal emulator core (~200 lines)
│   ├── commands.js     # Command handlers (~350 lines)
│   ├── preview.js      # Mock desktop animations (~250 lines)
│   └── demo.js         # Auto-demo sequence (~150 lines)
└── assets/
    └── favicon.svg     # Red terminal icon
```

## Deployment Steps

### 1. Enable GitHub Pages

1. Go to repository **Settings** → **Pages**
2. Under "Build and deployment":
   - Source: **GitHub Actions**
3. The workflow will automatically deploy on push to `main`

### 2. Configure Custom Domain DNS

Add a CNAME record in your DNS provider:

```
Type: CNAME
Name: cwm
Value: taulfsime.github.io
TTL: 3600 (or auto)
```

### 3. Verify Custom Domain

1. After DNS propagates (5-30 minutes), go to **Settings** → **Pages**
2. Under "Custom domain", verify `cwm.taulfsime.com` is shown
3. Check "Enforce HTTPS" once the certificate is issued

### 4. Test Deployment

After pushing to `main`:

1. Check **Actions** tab for the "Deploy to GitHub Pages" job
2. Once complete, visit https://cwm.taulfsime.com
3. Verify:
   - Auto-demo plays on load
   - Terminal accepts input after demo
   - Preview animations work
   - Sound toggle works (click to enable)
   - Mobile layout is responsive

## Terminal Commands

Users can type these commands in the terminal:

| Command | Description |
|---------|-------------|
| `help` | Show available commands |
| `install` | Show installation curl command |
| `demo` | Replay the auto-demo |
| `features` | List cwm features |
| `why` | Why use cwm? |
| `about` | About cwm |
| `github` | Open GitHub repository |
| `clear` | Clear terminal |
| `cwm --help` | Show cwm usage |
| `cwm focus --help` | Focus command help |
| `cwm maximize --help` | Maximize command help |
| `cwm move-display --help` | Move-display command help |
| `cwm resize --help` | Resize command help |

Simulated cwm commands also work:
- `cwm focus --app Safari` → focuses Safari in preview
- `cwm maximize` → maximizes focused window in preview
- `cwm move-display next` → moves window to Display 2
- `cwm resize 75` → resizes window to 75%

## Customization

### Colors

Edit CSS variables in `site/css/style.css`:

```css
:root {
  --accent-primary: #e63946;    /* Main red */
  --accent-secondary: #ff6b6b;  /* Lighter red */
  --bg-body: #0a0a0a;           /* Background */
  /* ... */
}
```

### Demo Sequence

Edit the sequence array in `site/js/demo.js`:

```javascript
const sequence = [
  { type: 'wait', duration: 800 },
  { type: 'print', text: 'Welcome...', class: 'highlight' },
  { type: 'type', text: 'cwm focus --app Slack' },
  // ...
];
```

### Adding Commands

Add new commands in `site/js/commands.js`:

```javascript
const commands = {
  help: cmdHelp,
  // add new command here
  mycommand: cmdMyCommand,
};

function cmdMyCommand(args) {
  Terminal.writeLine('My command output', 'success');
}
```

## Troubleshooting

### Pages not deploying

1. Check **Actions** tab for errors
2. Ensure `site/` folder exists with `index.html`
3. Verify workflow has `pages: write` permission

### Custom domain not working

1. Verify DNS CNAME record points to `taulfsime.github.io`
2. Check `site/CNAME` contains `cwm.taulfsime.com`
3. Wait for DNS propagation (up to 48 hours in rare cases)

### HTTPS certificate error

1. GitHub automatically provisions certificates
2. May take up to 24 hours after DNS is configured
3. Check **Settings** → **Pages** for certificate status

### Preview animations not working

1. Check browser console for JavaScript errors
2. Verify all JS files are loaded (Network tab)
3. Test in different browser

## Local Development

To test locally:

```bash
cd site
python3 -m http.server 8000
# or
npx serve .
```

Then open http://localhost:8000

## Browser Support

Tested and works in:
- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+

Mobile browsers are supported with stacked layout.
