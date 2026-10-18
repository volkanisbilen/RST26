-- The v2615 launcher expects the legacy LoginServer DOWNLOAD fields:
-- URL=<hostname> and PATH=<remote directory>. It does not accept an HTTP URL
-- with scheme/port in the URL field and closes after LS_DOWNLOADINFO_REQ.
UPDATE server_settings
SET patch_url = '127.0.0.1',
    patch_path = '/'
WHERE server_no = 1
  AND patch_url IN (
      'http://127.0.0.1:8080',
      'http://localhost:8080',
      'https://127.0.0.1:8080',
      'https://localhost:8080'
  );
