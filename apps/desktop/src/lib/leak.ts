import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "./platform";
import type { ExitIp } from "./servers";

export type Verdict = "ok" | "leak" | "unknown";

export interface LeakCheck {
  key: "ip" | "ipv6" | "dns";
  verdict: Verdict;
  note: string;
}

export interface Resolver {
  ip: string;
  country: string;
  countryName: string;
  asn: string;
}

export interface LeakReport {
  tun: boolean;
  vpn: ExitIp | null;
  real: ExitIp | null;
  ipv6: ExitIp | null;
  resolvers: Resolver[];
  checks: LeakCheck[];
}

export const leakTest = (): Promise<LeakReport> =>
  inTauri ? invoke<LeakReport>("leak_test") : Promise.reject("این کار فقط داخل برنامه‌ی GeekVPN انجام می‌شود.");
