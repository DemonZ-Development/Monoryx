export default {
  async fetch(request, env, ctx) {
    if (request.method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "GET, HEAD, POST, OPTIONS",
          "Access-Control-Allow-Headers": "Content-Type, Accept, User-Agent",
          "Access-Control-Max-Age": "86400",
        },
      });
    }

    if (request.method !== "GET" && request.method !== "HEAD" && request.method !== "POST") {
      return new Response(JSON.stringify({ error: "Method not allowed" }), {
        status: 405,
        headers: { "Content-Type": "application/json" },
      });
    }

    const apiKey = env.CURSEFORGE_API_KEY;
    if (!apiKey) {
      return new Response(
        JSON.stringify({ error: "Backend server API key is not configured." }),
        { status: 500, headers: { "Content-Type": "application/json" } }
      );
    }

    const url = new URL(request.url);
    let targetPath = url.pathname;
    if (targetPath.startsWith("/curseforge")) {
      targetPath = targetPath.slice(11);
    }
    if (!targetPath.startsWith("/v1")) {
      targetPath = "/v1" + targetPath;
    }
    const targetUrl = new URL("https://api.curseforge.com" + targetPath + url.search);

    const cacheKey = new Request(targetUrl.toString(), {
      method: "GET",
      headers: { "Accept": "application/json" },
    });
    const cache = caches.default;

    if (request.method === "GET") {
      const cached = await cache.match(cacheKey);
      if (cached) {
        const cachedHeaders = new Headers(cached.headers);
        cachedHeaders.set("X-Cache", "HIT");
        cachedHeaders.set("Access-Control-Allow-Origin", "*");
        return new Response(cached.body, {
          status: cached.status,
          statusText: cached.statusText,
          headers: cachedHeaders,
        });
      }
    }

    const upstreamHeaders = new Headers(request.headers);
    upstreamHeaders.set("x-api-key", apiKey);
    upstreamHeaders.set("Accept", "application/json");
    upstreamHeaders.set("User-Agent", "MONORYX-Official-Server/1.5.3");

    let upstreamResponse;
    try {
      upstreamResponse = await fetch(targetUrl.toString(), {
        method: request.method,
        headers: upstreamHeaders,
        body: request.method === "POST" ? await request.clone().arrayBuffer() : undefined,
      });
    } catch {
      return new Response(
        JSON.stringify({ error: "CurseForge service is currently unreachable." }),
        {
          status: 503,
          headers: {
            "Content-Type": "application/json",
            "Access-Control-Allow-Origin": "*",
          },
        }
      );
    }

    const responseHeaders = new Headers(upstreamResponse.headers);
    responseHeaders.set("Access-Control-Allow-Origin", "*");
    responseHeaders.delete("x-powered-by");

    if (request.method === "GET" && upstreamResponse.ok) {
      let ttl = 300;
      if (targetPath.includes("/description") || targetPath.match(/\/mods\/\d+$/)) {
        ttl = 3600;
      }
      responseHeaders.set("Cache-Control", `public, max-age=${ttl}`);
      responseHeaders.set("X-Cache", "MISS");

      const responseToCache = new Response(upstreamResponse.clone().body, {
        status: upstreamResponse.status,
        statusText: upstreamResponse.statusText,
        headers: responseHeaders,
      });
      ctx.waitUntil(cache.put(cacheKey, responseToCache));
    }

    return new Response(upstreamResponse.body, {
      status: upstreamResponse.status,
      statusText: upstreamResponse.statusText,
      headers: responseHeaders,
    });
  },
};
