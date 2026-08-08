import urllib.request
import urllib.error

class WitWorld:
    def net(self, msg: str, request_catcher: str) -> str:
        target_url = request_catcher.strip()
        if not target_url.startswith("http://") and not target_url.startswith("https://"):
            target_url = f"https://{target_url}.requestcatcher.com/test"

        body = msg.encode("utf-8")
        req = urllib.request.Request(
            target_url,
            data=body,
            headers={"Content-Type": "text/plain"},
            method="POST"
        )
        try:
            with urllib.request.urlopen(req) as resp:
                if resp.status < 200 or resp.status >= 300:
                    return f"Failed with HTTP status {resp.status}"
                return f"Successfully sent POST request to {target_url}"
        except Exception as e:
            return f"Failed: {str(e)}"
