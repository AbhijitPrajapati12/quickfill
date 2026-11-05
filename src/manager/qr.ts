import jsQR from "jsqr";

/** Reads a QR code from an image (a pasted screenshot or a picked file). */
export async function decodeQrImage(blob: Blob): Promise<string> {
  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(blob);
  } catch {
    throw "Couldn't open that image.";
  }
  const canvas = document.createElement("canvas");
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) throw "Couldn't read that image.";
  ctx.drawImage(bitmap, 0, 0);
  bitmap.close();
  const { data, width, height } = ctx.getImageData(0, 0, canvas.width, canvas.height);
  const result = jsQR(data, width, height, { inversionAttempts: "attemptBoth" });
  if (!result) throw "No QR code found in that image.";
  return result.data;
}

/** Checks a decoded QR is a 2FA setup link and returns it. */
export function toTotpLink(text: string): string {
  const lower = text.trim().toLowerCase();
  if (lower.startsWith("otpauth-migration://")) {
    throw "That's a Google Authenticator export code. Use the site's own 2FA setup QR instead.";
  }
  if (!lower.startsWith("otpauth://totp/")) throw "That QR isn't a 2FA setup code.";
  return text.trim();
}

/** A label for a 2FA link: its issuer, else the account name. */
export function labelFromLink(link: string): string {
  try {
    const url = new URL(link);
    const issuer = url.searchParams.get("issuer");
    if (issuer) return issuer;
    const path = decodeURIComponent(url.pathname.replace(/^\/+/, "")).replace(/^totp\//i, "");
    return path.split(":")[0].trim();
  } catch {
    return "";
  }
}
