// No passwords, decrypted identity material, or vault data enter the worker/cache.
if (!(window.__TAURI__ || window.__TAURI_INTERNALS__)) {
    const status = document.createElement('aside');
    status.setAttribute('role', 'status');
    status.style.cssText = 'position:fixed;bottom:0;left:0;right:0;z-index:1000;background:#17221b;color:#fff;padding:8px 16px;font:12px sans-serif';
    const text = document.createElement('span');
    text.textContent = 'Downloading application for offline use… Keep your encrypted vault backup.';
    status.append(text);
    const persist = document.createElement('button');
    persist.textContent = 'Keep browser storage';
    persist.style.marginLeft = '12px';
    persist.onclick = async () => {
        try {
            const granted = !!(navigator.storage?.persist && await navigator.storage.persist());
            persist.textContent = granted ? 'Persistent storage granted; backup still required' : 'Storage may be cleared; keep your backup';
        } catch (_) { persist.textContent = 'Storage persistence unavailable; keep your backup'; }
    };
    status.append(persist);
    document.body.append(status);
    async function readiness() {
        const worker = navigator.serviceWorker.controller;
        if (!worker) return;
        const channel = new MessageChannel();
        channel.port1.onmessage = event => {
            if (event.data.complete) text.textContent = 'Application cached for offline identity access. Chain sync/publication unavailable in this build. Keep your encrypted vault backup.';
            channel.port1.close();
        };
        worker.postMessage('offline-status', [channel.port2]);
    }
    if ('serviceWorker' in navigator && window.isSecureContext) {
        navigator.serviceWorker.addEventListener('controllerchange', readiness);
        navigator.serviceWorker.register('/sw.js', {updateViaCache: 'none'})
            .then(() => navigator.serviceWorker.ready).then(readiness)
            .catch(() => { text.textContent = 'Offline download failed; keep this page open and reconnect to retry.'; });
    } else text.textContent = 'Offline cache requires HTTPS or localhost and service-worker support.';
}
