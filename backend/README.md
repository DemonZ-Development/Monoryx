# MONORYX Backend Services Proxy

This folder contains the backend proxy worker that protects your CurseForge developer API key and caches catalog queries on edge servers.

## Quick Setup (Cloudflare Dashboard - 2 minutes)

1. Log into your free [Cloudflare Dashboard](https://dash.cloudflare.com/).
2. Navigate to **Compute (Workers & Pages)** -> **Create Application** -> **Create Worker**.
3. Name it `monoryx-services` (or whatever you prefer) and click **Deploy**.
4. Click **Edit code**, paste the entire contents of `curseforge-worker.js` into the editor, and click **Deploy**.
5. Go to your Worker's **Settings** -> **Variables and Secrets**.
6. Under **Secrets**, click **Add**:
   - Variable name: `CURSEFORGE_API_KEY`
   - Value: `<your-curseforge-api-key>`
   - Click **Save**.

Your backend proxy is now live, protecting your secret key and caching catalog searches.
