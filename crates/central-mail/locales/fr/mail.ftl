# The words of the mails Baylee Central sends (docs/email.md), in French.
#
# One message a line: `id = text`. `{ $name }` is replaced when a mail is written.

brand = Baylee
code-label = Votre code
code-secret = Ne transmettez ce code à personne. Personne chez Baylee ne vous le demande.
footer = Baylee ne vous demande jamais votre mot de passe par e-mail. Vous recevez cet e-mail parce que cette adresse a été indiquée pour un compte Baylee.

# The code that confirms an address.
confirm-subject = Confirmez votre adresse e-mail pour Baylee
confirm-title = Confirmez votre adresse e-mail
confirm-intro = Cette adresse a été indiquée pour un compte Baylee. Pour la confirmer, saisissez ce code là où vous avez indiqué l’adresse.
confirm-valid = Le code est valable { $minutes } minutes.
confirm-ignore = Si ce n’était pas vous, ignorez cet e-mail. Sans le code, l’adresse n’est pas utilisée.

# The reset code. `name` is the account's sign-in name.
reset-subject = Votre code de réinitialisation pour Baylee
reset-title = Définir un nouveau mot de passe
reset-intro = Un code de réinitialisation a été demandé pour le compte Baylee { $name }.
reset-how = Saisissez le code sur la page de réinitialisation pour définir un nouveau mot de passe.
reset-valid = Le code ne sert qu’une fois, pendant { $minutes } minutes.
reset-link = Ouvrir la page de réinitialisation
reset-ignore = Si ce n’était pas vous, ignorez cet e-mail. Votre mot de passe reste inchangé.

# The notice to an address that another one replaced.
replaced-subject = Votre adresse e-mail chez Baylee a été remplacée
replaced-title = Cette adresse a été remplacée
replaced-body = Cette adresse n’est plus l’adresse e-mail d’un compte Baylee. Une autre adresse a été confirmée à sa place.

# The notice to an address that was removed.
removed-subject = Votre adresse e-mail chez Baylee a été retirée
removed-title = Cette adresse a été retirée
removed-body = Cette adresse n’est plus l’adresse e-mail d’un compte Baylee. Elle a été retirée du compte.

# Both notices.
notice-check = Si ce n’était pas vous, connectez-vous et consultez la page de sécurité de votre compte.
notice-link = Ouvrir la page de sécurité

# The notices to the account's confirmed address that its login changed.
reset-done-subject = Le mot de passe de votre compte Baylee a été réinitialisé
reset-done-title = Un nouveau mot de passe a été défini
reset-done-body = Un code de réinitialisation a servi à définir un nouveau mot de passe pour le compte Baylee de cette adresse.
reset-done-kept = Les données du compte ont été conservées.
reset-done-new = Le compte repart avec un coffre vide. Ses données d’avant ne peuvent plus être ouvertes.
reset-done-sessions = Le compte est déconnecté partout : toutes ses sessions ont pris fin.
reset-done-not-you = Si ce n’était pas vous, quelqu’un d’autre peut maintenant se connecter à votre compte. Demandez un code de réinitialisation à un administrateur Baylee et définissez de nouveau votre propre mot de passe.
reset-done-recovery-kept = Si le compte a un code de récupération, il ouvre toujours les données. Gardez-le : avec lui, une réinitialisation conserve les données.
reset-done-recovery-new = Un code de récupération antérieur à cette réinitialisation n’ouvre plus rien.
password-subject = Le mot de passe de votre compte Baylee a été modifié
password-title = Le mot de passe a été modifié
password-body = Le mot de passe du compte Baylee de cette adresse a été modifié.
password-sessions = Toutes les autres sessions du compte ont été déconnectées.
password-not-you = Si ce n’était pas vous, quelqu’un d’autre connaît votre mot de passe. Définissez-en un nouveau avec un code de réinitialisation : demandez-en un sur la page de réinitialisation, ou demandez à un administrateur Baylee.
issued-subject = Un code de réinitialisation a été établi pour votre compte Baylee
issued-title = Un administrateur a établi un code de réinitialisation
issued-body = Un administrateur Baylee a établi un code de réinitialisation pour le compte Baylee de cette adresse. Avec lui, un nouveau mot de passe peut être défini pour le compte.
issued-valid = Le code ne sert qu’une fois, pendant { $hours } heures. Tant qu’il n’est pas utilisé, votre mot de passe fonctionne comme avant.
issued-not-you = Si vous ne l’avez pas demandé, connectez-vous et changez votre mot de passe sur la page de sécurité : cela met fin au code. Prévenez ensuite un administrateur.

# The confirmation mail's button, which opens the security page with the code filled in.
confirm-link = Confirmer l’adresse
