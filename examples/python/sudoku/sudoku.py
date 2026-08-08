class WitWorld:
    def solve(self, board_str: str) -> str:
        """Solve a sudoku puzzle given as an 81-char string (. for empty cells)."""
        if len(board_str) != 81:
            return "Invalid board length"

        board: list[list[int]] = []
        for i in range(9):
            row: list[int] = []
            for j in range(9):
                c = board_str[i * 9 + j]
                row.append(0 if c == "." else int(c))
            board.append(row)

        if self._solve(board):
            return "".join(str(board[i][j]) for i in range(9) for j in range(9))
        return "Unsolvable"

    def _solve(self, board: list[list[int]]) -> bool:
        for row in range(9):
            for col in range(9):
                if board[row][col] == 0:
                    for num in range(1, 10):
                        if self._is_valid(board, row, col, num):
                            board[row][col] = num
                            if self._solve(board):
                                return True
                            board[row][col] = 0
                    return False
        return True

    def _is_valid(self, board: list[list[int]], row: int, col: int, num: int) -> bool:
        for x in range(9):
            if board[row][x] == num or board[x][col] == num:
                return False
        sr, sc = (row // 3) * 3, (col // 3) * 3
        for i in range(3):
            for j in range(3):
                if board[sr + i][sc + j] == num:
                    return False
        return True
