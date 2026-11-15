(function () {
  if (window.Sewo) return;

  async function request(path, options) {
    const response = await fetch(path, options);
    if (!response.ok) {
      throw new Error(path + " -> " + response.status);
    }
    return response;
  }

  const Sewo = {
    async enter() {
      await request("/api/enter", { method: "POST" });
    },
    async status() {
      const response = await request("/api/status");
      return response.json();
    },
    async config() {
      const response = await request("/api/config");
      return response.json();
    },
    onStatus(callback, intervalMs) {
      let stopped = false;
      let timer = null;
      const interval = Math.max(200, intervalMs || 500);
      const tick = async () => {
        if (stopped) return;
        try {
          callback(await Sewo.status());
        } catch (_) {
          /* keep polling */
        }
        if (!stopped) timer = setTimeout(tick, interval);
      };
      tick();
      return function stop() {
        stopped = true;
        if (timer) clearTimeout(timer);
      };
    },
  };

  window.Sewo = Sewo;
})();
