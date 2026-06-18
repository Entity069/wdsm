export function solve(boardStr: string): string {
    if (boardStr.length !== 81) {
        return "Invalid board length";
    }

    const board: number[][] = [];
    for (let i = 0; i < 9; i++) {
        const row: number[] = [];
        for (let j = 0; j < 9; j++) {
            const char = boardStr[i * 9 + j];
            row.push(char === '.' ? 0 : parseInt(char, 10));
        }
        board.push(row);
    }

    if (solveSudoku(board)) {
        let result = "";
        for (let i = 0; i < 9; i++) {
            for (let j = 0; j < 9; j++) {
                result += board[i][j].toString();
            }
        }
        return result;
    }

    return "Unsolvable";
}

function solveSudoku(board: number[][]): boolean {
    for (let row = 0; row < 9; row++) {
        for (let col = 0; col < 9; col++) {
            if (board[row][col] === 0) {
                for (let num = 1; num <= 9; num++) {
                    if (isValid(board, row, col, num)) {
                        board[row][col] = num;
                        if (solveSudoku(board)) {
                            return true;
                        }
                        board[row][col] = 0;
                    }
                }
                return false;
            }
        }
    }
    return true;
}

function isValid(board: number[][], row: number, col: number, num: number): boolean {
    for (let x = 0; x < 9; x++) {
        if (board[row][x] === num || board[x][col] === num) {
            return false;
        }
    }
    const startRow = Math.floor(row / 3) * 3;
    const startCol = Math.floor(col / 3) * 3;
    for (let i = 0; i < 3; i++) {
        for (let j = 0; j < 3; j++) {
            if (board[i + startRow][j + startCol] === num) {
                return false;
            }
        }
    }
    return true;
}
