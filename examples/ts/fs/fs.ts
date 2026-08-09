import * as fs from 'fs';
import * as path from 'path';

export function readFile(filename: string): string {
    const filePath = path.join('/data', filename);
    try {
        return fs.readFileSync(filePath, 'utf8');
    } catch (err: any) {
        return `Error reading file: ${err.message}`;
    }
}
