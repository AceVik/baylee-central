// The latest release (from GitHub, cached in this tab) and the visitor's
// machine, for the download button and the platform cards.

import { useEffect, useMemo, useState } from "react";

import { RELEASES_PATH, useGitHub } from "../github";
import { detect, platformFor, type Machine } from "./machine";
import { pickRelease, sortFiles, type Files, type Platform, type Release } from "./release";

export interface ReleaseState {
  /** `undefined` while asking GitHub, `null` when it did not answer. */
  release: Release | null | undefined;
  files: Files;
  machine: Machine | null;
  mine: Platform | null;
}

export function useRelease(): ReleaseState {
  const release = useGitHub(RELEASES_PATH, "releases", pickRelease);
  const [machine, setMachine] = useState<Machine | null>(null);
  useEffect(() => {
    let live = true;
    void detect().then((m) => {
      if (live) setMachine(m);
    });
    return () => {
      live = false;
    };
  }, []);
  const files = useMemo(() => (release ? sortFiles(release) : {}), [release]);
  return { release, files, machine, mine: platformFor(machine) };
}
