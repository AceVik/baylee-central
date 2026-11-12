# The words of the mails Baylee Central sends (docs/email.md), in Italian.
#
# One message a line: `id = text`. `{ $name }` is replaced when a mail is written.

brand = Baylee
code-label = Il tuo codice
code-secret = Non passare questo codice a nessuno. Nessuno di Baylee te lo chiede.
footer = Baylee non ti chiede mai la password per e-mail. Ricevi questa e-mail perché questo indirizzo è stato indicato per un account Baylee.

# The code that confirms an address.
confirm-subject = Conferma il tuo indirizzo e-mail per Baylee
confirm-title = Conferma il tuo indirizzo e-mail
confirm-intro = Questo indirizzo è stato indicato per un account Baylee. Per confermarlo, inserisci questo codice dove hai indicato l'indirizzo.
confirm-valid = Il codice vale { $minutes } minuti.
confirm-ignore = Se non eri tu, ignora questa e-mail. Senza il codice l'indirizzo non viene usato.

# The reset code. `name` is the account's sign-in name.
reset-subject = Il tuo codice di reimpostazione per Baylee
reset-title = Imposta una nuova password
reset-intro = È stato richiesto un codice di reimpostazione per l'account Baylee { $name }.
reset-how = Inserisci il codice nella pagina di reimpostazione per impostare una nuova password.
reset-valid = Il codice vale una sola volta, per { $minutes } minuti.
reset-link = Apri la pagina di reimpostazione
reset-ignore = Se non eri tu, ignora questa e-mail. La tua password resta com'è.

# The notice to an address that another one replaced.
replaced-subject = Il tuo indirizzo e-mail su Baylee è stato sostituito
replaced-title = Questo indirizzo è stato sostituito
replaced-body = Questo indirizzo non è più l'indirizzo e-mail di un account Baylee. Al suo posto è stato confermato un altro indirizzo.

# The notice to an address that was removed.
removed-subject = Il tuo indirizzo e-mail su Baylee è stato rimosso
removed-title = Questo indirizzo è stato rimosso
removed-body = Questo indirizzo non è più l'indirizzo e-mail di un account Baylee. È stato rimosso dall'account.

# Both notices.
notice-check = Se non eri tu, accedi e controlla la pagina di sicurezza del tuo account.
notice-link = Apri la pagina di sicurezza

# The notices to the account's confirmed address that its login changed.
# Not translated yet: English until a translation replaces these lines.
reset-done-subject = The password of your Baylee account was reset
reset-done-title = A new password was set
reset-done-body = A reset code was used to set a new password for the Baylee account of this address.
reset-done-kept = The account's data was kept.
reset-done-new = The account starts with an empty vault. Its data from before cannot be opened any more.
reset-done-sessions = The account is signed out everywhere: every session it had was ended.
reset-done-not-you = If this was not you, someone else can sign in to your account now. Ask a Baylee administrator for a reset code and set your own password again.
reset-done-recovery-kept = Your recovery code still opens your data. Keep it: with it, a reset keeps the data.
reset-done-recovery-new = A recovery code from before this reset opens nothing any more.
password-subject = The password of your Baylee account was changed
password-title = The password was changed
password-body = The password of the Baylee account of this address was changed.
password-sessions = Every other session of the account was signed out.
password-not-you = If this was not you, someone else knows your password. Set a new one with a reset code: ask for one on the reset page, or ask a Baylee administrator.
issued-subject = A reset code was made for your Baylee account
issued-title = An administrator made a reset code
issued-body = A Baylee administrator made a reset code for the Baylee account of this address. With it, a new password can be set for the account.
issued-valid = The code works once, for { $hours } hours. Until it is used, your password works as it did.
issued-not-you = If you did not ask for it, sign in and change your password on the security page: that ends the code. Then tell an administrator.
