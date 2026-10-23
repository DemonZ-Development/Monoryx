export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);

    if (request.method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "GET, HEAD, POST, OPTIONS",
          "Access-Control-Allow-Headers": "Content-Type, Accept, Authorization, X-Requested-With",
          "Access-Control-Max-Age": "86400",
        },
      });
    }

    if (url.pathname === "/" || url.pathname === "/health") {
      return jsonResponse({
        status: "ok",
        service: "dzd-skins",
        version: "1.0.0",
      });
    }

    if (request.method === "POST" && (url.pathname === "/upload" || url.pathname === "/api/skins/upload")) {
      return handleUpload(request, env);
    }

    if (request.method === "GET" || request.method === "HEAD") {
      return handleGetTexture(request, env, url);
    }

    return jsonResponse({ error: "Method not allowed" }, 405);
  },
};

async function handleUpload(request, env) {
  let pngBuffer;
  let model = "classic";
  const contentType = request.headers.get("content-type") || "";

  if (contentType.includes("multipart/form-data")) {
    const formData = await request.formData();
    const file = formData.get("file") || formData.get("skin");
    if (!file) {
      return jsonResponse({ error: "Missing file field in multipart form" }, 400);
    }
    pngBuffer = await file.arrayBuffer();
    const modelParam = formData.get("model");
    if (typeof modelParam === "string" && modelParam.toLowerCase() === "slim") {
      model = "slim";
    }
  } else {
    pngBuffer = await request.arrayBuffer();
    const url = new URL(request.url);
    if (url.searchParams.get("model") === "slim" || url.searchParams.get("slim") === "true") {
      model = "slim";
    }
  }

  if (!pngBuffer || pngBuffer.byteLength === 0) {
    return jsonResponse({ error: "Empty payload received" }, 400);
  }

  const bytes = new Uint8Array(pngBuffer);
  if (!isValidPng(bytes)) {
    return jsonResponse({ error: "Uploaded file is not a valid Minecraft skin PNG (must be 64x64 or 64x32)" }, 400);
  }

  const hashBuffer = await crypto.subtle.digest("SHA-256", bytes);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hash = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");

  await env.SKINS.put(`skin:${hash}`, bytes, {
    metadata: {
      model,
      uploadedAt: Date.now(),
      size: bytes.byteLength,
    },
  });

  const skinUrl = `https://skins.demonz.org/textures/${hash}.png`;
  const command = `/skin url ${skinUrl} ${model}`;

  return jsonResponse({
    success: true,
    url: skinUrl,
    hash,
    model,
    command,
  });
}

function isValidPng(bytes) {
  if (bytes.length < 24) {
    return false;
  }

  const header = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  for (let i = 0; i < 8; i++) {
    if (bytes[i] !== header[i]) {
      return false;
    }
  }

  const width = (bytes[16] << 24) | (bytes[17] << 16) | (bytes[18] << 8) | bytes[19];
  const height = (bytes[20] << 24) | (bytes[21] << 16) | (bytes[22] << 8) | bytes[23];

  return width === 64 && (height === 64 || height === 32);
}

async function handleGetTexture(request, env, url) {
  let key = url.pathname.replace(/^\/+(textures|skins|raw|api\/skins)?\/+/, "").replace(/^\/+/, "");
  if (key.endsWith(".png")) {
    key = key.slice(0, -4);
  }

  if (!key || !/^[a-f0-9]{16,64}$/i.test(key)) {
    return jsonResponse({ error: "Texture not found" }, 404);
  }

  const raw = await env.SKINS.get(`skin:${key.toLowerCase()}`, { type: "arrayBuffer" });
  if (!raw) {
    return jsonResponse({ error: "Texture not found" }, 404);
  }

  if (request.method === "HEAD") {
    return new Response(null, {
      status: 200,
      headers: {
        "Content-Type": "image/png",
        "Content-Length": raw.byteLength.toString(),
        "Cache-Control": "public, max-age=31536000, immutable",
        "Access-Control-Allow-Origin": "*",
      },
    });
  }

  return new Response(raw, {
    status: 200,
    headers: {
      "Content-Type": "image/png",
      "Content-Length": raw.byteLength.toString(),
      "Cache-Control": "public, max-age=31536000, immutable",
      "Access-Control-Allow-Origin": "*",
      "Content-Disposition": `inline; filename="${key}.png"`,
    },
  });
}

function jsonResponse(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "Content-Type": "application/json",
      "Access-Control-Allow-Origin": "*",
    },
  });
}
