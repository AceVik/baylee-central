// /account/contacts: the messenger with its contacts in front.
import { MessengerPage } from "../chat/ChatPage";
import { useTitle } from "../i18n";

export default function ContactsPage() {
  useTitle("title.accountContacts");
  return <MessengerPage tab="contacts" />;
}
