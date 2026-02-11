const express = require('express');
const crypto = require('crypto');
const zlib = require('zlib');
const tar = require('tar-stream');

const app = express();
app.use(express.json());

const PORT = process.env.MOCK_SERVER_PORT || 8080;
const BASE_URL = process.env.MOCK_SERVER_BASE_URL || `http://mock-github:${PORT}`;

// current test scenario
let scenario = {
  mode: 'normal',
  customReleases: []
};

// health check
app.get('/health', (req, res) => {
  res.send('OK');
});

// GitHub API: list releases
app.get('/repos/:owner/:repo/releases', (req, res) => {
  console.log(`[${new Date().toISOString()}] GET /repos/${req.params.owner}/${req.params.repo}/releases`);

  if (scenario.mode === 'rate_limited') {
    return res.status(403)
      .set('x-ratelimit-remaining', '0')
      .set('x-ratelimit-reset', '9999999999')
      .send('Rate limit exceeded');
  }

  if (scenario.mode === 'no_releases') {
    return res.json([]);
  }

  const releases = scenario.customReleases.length > 0 
    ? scenario.customReleases 
    : generateDefaultReleases();

  res.json(releases);
});

// download binary archive
app.get('/download/:filename', async (req, res) => {
  const { filename } = req.params;
  console.log(`[${new Date().toISOString()}] GET /download/${filename}`);

  if (scenario.mode === 'slow_download') {
    await new Promise(resolve => setTimeout(resolve, 5000));
  }

  if (filename.endsWith('.sha256')) {
    const baseFilename = filename.replace('.sha256', '');
    const binaryData = await generateTestBinary(baseFilename);
    
    let checksum;
    if (scenario.mode === 'checksum_mismatch') {
      checksum = '0'.repeat(64);
    } else {
      checksum = crypto.createHash('sha256').update(binaryData).digest('hex');
    }
    
    return res.type('text/plain').send(`${checksum}  ${baseFilename}`);
  }

  if (filename.endsWith('.tar.gz')) {
    if (scenario.mode === 'corrupt_download') {
      return res.type('application/gzip').send(Buffer.alloc(1024));
    }

    const binaryData = await generateTestBinary(filename);
    return res.type('application/gzip').send(binaryData);
  }

  res.status(404).send('File not found');
});

// control: set scenario
app.post('/control/set-scenario', (req, res) => {
  const { mode, custom_releases } = req.body;
  
  const validModes = ['normal', 'checksum_mismatch', 'corrupt_download', 'slow_download', 'rate_limited', 'no_releases', 'binary_test_fails'];
  
  if (!validModes.includes(mode)) {
    return res.status(400).send(`Unknown mode: ${mode}`);
  }

  scenario.mode = mode;
  scenario.customReleases = custom_releases || [];
  
  console.log(`[${new Date().toISOString()}] Scenario set to: ${mode}`);
  res.send('Scenario updated');
});

// control: reset scenario
app.post('/control/reset', (req, res) => {
  scenario = { mode: 'normal', customReleases: [] };
  console.log(`[${new Date().toISOString()}] Scenario reset to default`);
  res.send('Scenario reset');
});

// control: get status
app.get('/control/status', (req, res) => {
  res.json({
    mode: scenario.mode,
    custom_releases_count: scenario.customReleases.length
  });
});

function generateDefaultReleases() {
  const architectures = ['x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu'];

  const releases = [];

  // stable release (oldest)
  const stableCommit = 'a1b2c3d4';
  const stableDate = '20260201';
  releases.push({
    tag_name: `stable-${stableCommit}`,
    name: `Stable Release ${stableCommit} (${stableDate})`,
    body: 'Stable release for testing',
    prerelease: false,
    created_at: '2026-02-01T00:00:00Z',
    assets: architectures.map(arch => ({
      name: `cwm-stable-${arch}-${stableDate}.tar.gz`,
      browser_download_url: `${BASE_URL}/download/cwm-stable-${arch}-${stableDate}.tar.gz`,
      size: 1024 * 1024
    }))
  });

  // beta release (middle)
  const betaCommit = 'b2c3d4e5';
  const betaDate = '20260205';
  releases.push({
    tag_name: `beta-${betaCommit}`,
    name: `Beta Release ${betaCommit} (${betaDate})`,
    body: 'Beta release for testing',
    prerelease: true,
    created_at: '2026-02-05T00:00:00Z',
    assets: architectures.map(arch => ({
      name: `cwm-beta-${arch}-${betaDate}.tar.gz`,
      browser_download_url: `${BASE_URL}/download/cwm-beta-${arch}-${betaDate}.tar.gz`,
      size: 1024 * 1024
    }))
  });

  // dev release (newest)
  const devCommit = 'c3d4e5f6';
  const devDate = '20260210';
  releases.push({
    tag_name: `dev-${devCommit}`,
    name: `Development Build ${devCommit} (${devDate})`,
    body: 'Dev release for testing',
    prerelease: true,
    created_at: '2026-02-10T00:00:00Z',
    assets: architectures.map(arch => ({
      name: `cwm-dev-${arch}-${devDate}.tar.gz`,
      browser_download_url: `${BASE_URL}/download/cwm-dev-${arch}-${devDate}.tar.gz`,
      size: 1024 * 1024
    }))
  });

  return releases;
}

async function generateTestBinary(filename) {
  const base = filename.replace('.tar.gz', '');
  const parts = base.split('-');
  const channel = parts[1] || 'dev';
  const date = parts[parts.length - 1] || '20260101';

  const commits = {
    stable: 'a1b2c3d4',
    beta: 'b2c3d4e5',
    dev: 'c3d4e5f6'
  };
  const commit = commits[channel] || '00000000';

  let scriptContent;
  if (scenario.mode === 'binary_test_fails') {
    scriptContent = '#!/bin/sh\nexit 1\n';
  } else {
    scriptContent = `#!/bin/sh
case "$1" in
    --version|version)
        echo "cwm ${commit} (${channel}, ${date})"
        exit 0
        ;;
    --help|-h)
        echo "cwm - test binary"
        exit 0
        ;;
    *)
        echo "cwm test binary"
        exit 0
        ;;
esac
`;
  }

  return createTarGz('cwm', scriptContent, 0o755);
}

function createTarGz(filename, content, mode) {
  const pack = tar.pack();
  
  pack.entry({ name: filename, mode: mode }, content);
  pack.finalize();

  const chunks = [];
  const gzip = zlib.createGzip();
  
  return new Promise((resolve, reject) => {
    pack.pipe(gzip);
    gzip.on('data', chunk => chunks.push(chunk));
    gzip.on('end', () => resolve(Buffer.concat(chunks)));
    gzip.on('error', reject);
  });
}

app.listen(PORT, '0.0.0.0', () => {
  console.log(`[${new Date().toISOString()}] Mock GitHub server listening on port ${PORT}`);
  console.log(`[${new Date().toISOString()}] Base URL: ${BASE_URL}`);
});
