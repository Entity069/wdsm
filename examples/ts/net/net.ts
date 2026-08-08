export async function net(msg: string, request_catcher: string): Promise<string> {
    let targetUrl = request_catcher.trim();
    if (!targetUrl.startsWith("http://") && !targetUrl.startsWith("https://")) {
        targetUrl = `https://${targetUrl}.requestcatcher.com/test`;
    }

    try {
        const res = await fetch(targetUrl, {
            method: "POST",
            headers: {
                "Content-Type": "text/plain"
            },
            body: msg
        });

        if (!res.ok) {
            return `Failed with HTTP status ${res.status}`;
        }
        return `Successfully sent POST request to ${targetUrl}`;
    } catch (err: any) {
        return `Failed: ${err?.message || String(err)}`;
    }
}
