const http = require('http');
const fs = require('fs');
const path = require('path');

const root = 'F:/ArkCodes/国标算番器';
const types = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.svg': 'image/svg+xml'
};

http.createServer((req, res) => {
  let p = decodeURIComponent(req.url.split('?')[0]);
  if (p === '/') p = '/mockup.html';
  const f = path.join(root, p);
  fs.readFile(f, (e, d) => {
    if (e) { res.statusCode = 404; res.end('404'); return; }
    res.setHeader('Content-Type', types[path.extname(f).toLowerCase()] || 'application/octet-stream');
    res.end(d);
  });
}).listen(8791, () => console.log('serving on 8791'));
