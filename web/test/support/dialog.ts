// jsdom draws a <dialog> but cannot open one modally: these stand-ins open
// and close it by its attribute, which is all the components read.

function showModal(this: HTMLDialogElement): void {
  this.setAttribute("open", "");
}

function close(this: HTMLDialogElement): void {
  this.removeAttribute("open");
}

export function standInDialogs(): void {
  const proto = HTMLDialogElement.prototype as HTMLDialogElement & { showModal?: () => void; close?: () => void };
  if (typeof proto.showModal !== "function") proto.showModal = showModal;
  if (typeof proto.close !== "function") proto.close = close;
}
