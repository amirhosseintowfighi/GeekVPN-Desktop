const DIGITS = "۰۱۲۳۴۵۶۷۸۹";

/** Western digits to Persian, and the decimal point to the Persian «٫». */
export function faDigits(value: string | number): string {
  return String(value).replace(/[0-9.]/g, (c) => (c === "." ? "٫" : DIGITS[Number(c)]!));
}
