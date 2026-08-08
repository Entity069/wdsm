import http.client
import encodings.idna

class WitWorld:
    def net(self, msg: str, request_catcher: str) -> str:
        target = request_catcher.strip()
        if target.startswith("http://"):
            target = target[7:]
        elif target.startswith("https://"):
            target = target[8:]

        if "/" in target:
            host, path = target.split("/", 1)
            path = "/" + path
        else:
            host = f"{target}.requestcatcher.com" if "." not in target else target
            path = "/test123"

        try:
            conn = http.client.HTTPConnection(host, timeout=10)
            headers = {"Content-Type": "text/plain"}
            body = msg.encode("utf-8")
            conn.request("POST", path, body=body, headers=headers)
            resp = conn.getresponse()
            status = resp.status
            conn.close()

            if status >= 200 and status < 400:
                return f"Successfully sent POST request to http://{host}{path}"
            return f"Failed with HTTP status {status}"
        except Exception as e:
            return f"Failed: {str(e)}"
