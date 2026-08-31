export function excelColName(index: number): string {
  let n = index
  let result = ''
  while (n >= 0) {
    result = String.fromCharCode((n % 26) + 65) + result
    n = Math.floor(n / 26) - 1
  }
  return result
}