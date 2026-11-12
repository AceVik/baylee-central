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
reset-done-subject = La password del tuo account Baylee è stata reimpostata
reset-done-title = È stata impostata una nuova password
reset-done-body = Un codice di reimpostazione è stato usato per impostare una nuova password per l'account Baylee di questo indirizzo.
reset-done-kept = I dati dell'account sono stati conservati.
reset-done-new = L'account riparte con una cassaforte vuota. I suoi dati di prima non si possono più aprire.
reset-done-sessions = L'account è disconnesso ovunque: tutte le sue sessioni sono state terminate.
reset-done-not-you = Se non eri tu, ora qualcun altro può accedere al tuo account. Chiedi a un amministratore di Baylee un codice di reimpostazione e imposta di nuovo la tua password.
reset-done-recovery-kept = Se l'account ha un codice di recupero, apre ancora i dati. Conservalo: con quello, una reimpostazione conserva i dati.
reset-done-recovery-new = Un codice di recupero precedente a questa reimpostazione non apre più nulla.
password-subject = La password del tuo account Baylee è stata cambiata
password-title = La password è stata cambiata
password-body = La password dell'account Baylee di questo indirizzo è stata cambiata.
password-sessions = Tutte le altre sessioni dell'account sono state disconnesse.
password-not-you = Se non eri tu, qualcun altro conosce la tua password. Impostane una nuova con un codice di reimpostazione: richiedilo nella pagina di reimpostazione oppure chiedilo a un amministratore di Baylee.
issued-subject = È stato creato un codice di reimpostazione per il tuo account Baylee
issued-title = Un amministratore ha creato un codice di reimpostazione
issued-body = Un amministratore di Baylee ha creato un codice di reimpostazione per l'account Baylee di questo indirizzo. Con questo codice si può impostare una nuova password per l'account.
issued-valid = Il codice vale una sola volta, per { $hours } ore. Finché non viene usato, la tua password funziona come prima.
issued-not-you = Se non l'hai chiesto tu, accedi e cambia la password nella pagina di sicurezza: così il codice termina. Poi avvisa un amministratore.

# The confirmation mail's button, which opens the security page with the code filled in.
confirm-link = Conferma indirizzo
