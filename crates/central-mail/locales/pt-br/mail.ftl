# The words of the mails Baylee Central sends (docs/email.md), in Brazilian Portuguese.
#
# One message a line: `id = text`. `{ $name }` is replaced when a mail is written.

brand = Baylee
code-label = Seu código
code-secret = Não repasse este código. Ninguém do Baylee pede por ele.
footer = O Baylee nunca pede a sua senha por e-mail. Você recebe este e-mail porque este endereço foi indicado para uma conta do Baylee.

# O código que confirma um endereço.
confirm-subject = Confirme seu e-mail no Baylee
confirm-title = Confirme seu endereço de e-mail
confirm-intro = Este endereço foi indicado para uma conta do Baylee. Para confirmá-lo, digite este código onde você indicou o endereço.
confirm-valid = O código vale por { $minutes } minutos.
confirm-ignore = Se não foi você, ignore este e-mail. O endereço não é usado sem o código.

# O código de redefinição. `name` é o nome de entrada da conta.
reset-subject = Seu código de redefinição do Baylee
reset-title = Definir uma nova senha
reset-intro = Um código de redefinição foi pedido para a conta do Baylee { $name }.
reset-how = Digite o código na página de redefinição para definir uma nova senha.
reset-valid = O código vale uma única vez, por { $minutes } minutos.
reset-link = Abrir a página de redefinição
reset-ignore = Se não foi você, ignore este e-mail. Sua senha continua como está.

# O aviso a um endereço que foi substituído por outro.
replaced-subject = Seu e-mail no Baylee foi substituído
replaced-title = Este endereço foi substituído
replaced-body = Este endereço não é mais o endereço de e-mail de uma conta do Baylee. Outro endereço foi confirmado no lugar dele.

# O aviso a um endereço que foi removido.
removed-subject = Seu e-mail no Baylee foi removido
removed-title = Este endereço foi removido
removed-body = Este endereço não é mais o endereço de e-mail de uma conta do Baylee. Ele foi removido da conta.

# Os dois avisos.
notice-check = Se não foi você, entre e confira a página de segurança da sua conta.
notice-link = Abrir a página de segurança

# The notices to the account's confirmed address that its login changed.
# Not translated yet: English until a translation replaces these lines.
reset-done-subject = The password of your Baylee account was reset
reset-done-title = A new password was set
reset-done-body = A reset code was used to set a new password for the Baylee account of this address.
reset-done-kept = The account's data was kept.
reset-done-new = The account starts with an empty vault. Its data from before cannot be opened any more.
reset-done-sessions = The account is signed out everywhere: every session it had was ended.
reset-done-not-you = If this was not you, someone else can sign in to your account now. Ask a Baylee administrator for a reset code and set your own password again.
reset-done-recovery-kept = If the account has a recovery code, it still opens the data. Keep it: with it, a reset keeps the data.
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

# English for now: to be translated.
# The confirmation mail's button, which opens the security page with the code filled in.
confirm-link = Confirm address
