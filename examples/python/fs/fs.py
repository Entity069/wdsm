import os

class WitWorld:
    def read_file(self, filename: str) -> str:
        path = os.path.join("/data", filename)
        try:
            with open(path, "r") as f:
                return f.read()
        except Exception as e:
            return f"Error reading file: {e}"

