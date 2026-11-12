# Die Worte der Mails, die Baylee Central verschickt (docs/email.md).
#
# Eine Meldung je Zeile: `id = Text`. `{ $name }` wird beim Schreiben einer
# Mail ersetzt. Eine Zeile, die mit `#` beginnt, ist ein Kommentar. Keine
# Auszeichnung: Die Worte stehen im Klartext, wie sie sind, und im HTML
# maskiert.

brand = Baylee
code-label = Dein Code
code-secret = Gib diesen Code nicht weiter. Niemand von Baylee fragt danach.
footer = Baylee fragt nie per E-Mail nach deinem Passwort. Du bekommst diese Mail, weil diese Adresse für einen Baylee-Account angegeben wurde.

# Der Code, der eine Adresse bestätigt.
confirm-subject = Bestätige deine E-Mail-Adresse für Baylee
confirm-title = Bestätige deine E-Mail-Adresse
confirm-intro = Diese Adresse wurde für einen Baylee-Account angegeben. Um sie zu bestätigen, gib diesen Code dort ein, wo du die Adresse angegeben hast.
confirm-valid = Der Code gilt { $minutes } Minuten.
confirm-ignore = Wenn du das nicht warst, ignoriere diese Mail. Ohne den Code wird die Adresse nicht verwendet.

# Der Reset-Code. `name` ist der Anmeldename des Accounts.
reset-subject = Dein Reset-Code für Baylee
reset-title = Neues Passwort setzen
reset-intro = Für den Baylee-Account { $name } wurde ein Reset-Code angefordert.
reset-how = Gib den Code auf der Reset-Seite ein, um ein neues Passwort zu setzen.
reset-valid = Der Code gilt einmal, { $minutes } Minuten lang.
reset-link = Reset-Seite öffnen
reset-ignore = Wenn du das nicht warst, ignoriere diese Mail. Dein Passwort bleibt, wie es ist.

# Der Hinweis an eine Adresse, die durch eine andere ersetzt wurde.
replaced-subject = Deine E-Mail-Adresse bei Baylee wurde ersetzt
replaced-title = Diese Adresse wurde ersetzt
replaced-body = Diese Adresse ist nicht mehr die E-Mail-Adresse eines Baylee-Accounts. An ihrer Stelle wurde eine andere Adresse bestätigt.

# Der Hinweis an eine Adresse, die entfernt wurde.
removed-subject = Deine E-Mail-Adresse bei Baylee wurde entfernt
removed-title = Diese Adresse wurde entfernt
removed-body = Diese Adresse ist nicht mehr die E-Mail-Adresse eines Baylee-Accounts. Sie wurde aus dem Account entfernt.

# Beide Hinweise.
notice-check = Wenn du das nicht warst, melde dich an und sieh auf der Sicherheitsseite deines Accounts nach.
notice-link = Sicherheitsseite öffnen
