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
confirm-link = Adresse bestätigen
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

# Die Hinweise an die bestätigte Adresse des Accounts, dass sich seine
# Anmeldung geändert hat. Sie nennen keinen Account: Der Anmeldename ist die
# Hälfte dessen, was ein Reset per E-Mail braucht, und diese Mails gehen
# hinaus, ohne dass jemand nach ihnen gefragt hat.

# Ein Reset-Code hat ein neues Passwort gesetzt.
reset-done-subject = Das Passwort deines Baylee-Accounts wurde zurückgesetzt
reset-done-title = Ein neues Passwort wurde gesetzt
reset-done-body = Mit einem Reset-Code wurde ein neues Passwort für den Baylee-Account dieser Adresse gesetzt.
reset-done-kept = Die Daten des Accounts sind geblieben.
reset-done-new = Der Account fängt mit einem leeren Tresor an. Seine bisherigen Daten lassen sich nicht mehr öffnen.
reset-done-sessions = Der Account ist überall abgemeldet: Jede seiner Sitzungen wurde beendet.
reset-done-not-you = Wenn du das nicht warst, kann sich jetzt jemand anderes bei deinem Account anmelden. Bitte einen Administrator von Baylee um einen Reset-Code und setze wieder dein eigenes Passwort.
reset-done-recovery-kept = Hat der Account einen Wiederherstellungscode, öffnet er die Daten weiterhin. Bewahre ihn auf: Mit ihm bleiben die Daten bei einem Reset erhalten.
reset-done-recovery-new = Ein Wiederherstellungscode von vor diesem Reset öffnet nichts mehr.

# Das Passwort wurde geändert.
password-subject = Das Passwort deines Baylee-Accounts wurde geändert
password-title = Das Passwort wurde geändert
password-body = Das Passwort des Baylee-Accounts dieser Adresse wurde geändert.
password-sessions = Alle anderen Sitzungen des Accounts wurden abgemeldet.
password-not-you = Wenn du das nicht warst, kennt jemand anderes dein Passwort. Setze mit einem Reset-Code ein neues: Fordere einen auf der Reset-Seite an oder bitte einen Administrator von Baylee darum.

# Ein Administrator hat einen Reset-Code ausgestellt. `hours` ist, wie lange er gilt.
issued-subject = Für deinen Baylee-Account wurde ein Reset-Code ausgestellt
issued-title = Ein Administrator hat einen Reset-Code ausgestellt
issued-body = Ein Administrator von Baylee hat für den Baylee-Account dieser Adresse einen Reset-Code ausgestellt. Mit ihm lässt sich ein neues Passwort für den Account setzen.
issued-valid = Der Code gilt einmal, { $hours } Stunden lang. Bis er benutzt wird, gilt dein Passwort wie bisher.
issued-not-you = Wenn du ihn nicht angefordert hast, melde dich an und ändere dein Passwort auf der Sicherheitsseite: Damit endet der Code. Sag danach einem Administrator Bescheid.

# Sicherung des Wiederherstellungscodes als ZIP-Anhang. `name` ist das Handle oder der Anmeldename.
recovery-backup-subject = Deine Sicherung des Baylee-Wiederherstellungscodes
recovery-backup-title = Sicherung deines Wiederherstellungscodes
recovery-backup-intro = Im Anhang findest du die Sicherung des Wiederherstellungscodes für das Baylee-Konto { $name }.
recovery-backup-encrypted = Das Archiv baylee-recovery-code.zip ist mit dem von dir gewählten Passwort verschlüsselt.
recovery-backup-plain = Das Archiv baylee-recovery-code.zip wurde unverschlüsselt versendet. Bewahre es sicher auf und lösche diese Mail nach einer Offline-Sicherung.
recovery-backup-fact-file = Bewahre das Archiv an einem sicheren Ort getrennt von diesem Postfach auf.
recovery-backup-fact-reset = Wer sowohl diesen Code als auch ein Passwort-Reset an diese Adresse nutzt, kann deine verschlüsselten Daten öffnen.
recovery-backup-warning = Wenn du diese Sicherung nicht angefordert hast, ignoriere diese Mail oder prüfe deine Kontoeinstellungen.
recovery-backup-link = Konto öffnen

# Ein Spieler hat den Empfänger auf Baylee als Freund eingeladen. `name` ist
# der öffentliche Name des Einladenden; `days` ist, wie viele Tage der Link gilt.
invite-subject = Baylee-Freundschaftseinladung
invite-title = Freundschaftseinladung
invite-intro = { $name } hat dich auf Baylee als Freund eingeladen.
invite-note-label = Nachricht:
invite-fact-friend = Wenn du diesen Einmal-Link annimmst, werdet ihr auf Baylee befreundet.
invite-valid-one = Der Einladungslink ist 1 Tag lang gültig.
invite-valid = Der Einladungslink ist { $days } Tage lang gültig.
invite-warning = Falls du diese Einladung nicht erwartet hast, kannst du diese Mail ignorieren.
invite-link = Einladung öffnen
