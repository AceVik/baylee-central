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
reset-done-subject = A senha da sua conta do Baylee foi redefinida
reset-done-title = Uma nova senha foi definida
reset-done-body = Um código de redefinição foi usado para definir uma nova senha para a conta do Baylee deste endereço.
reset-done-kept = Os dados da conta foram mantidos.
reset-done-new = A conta recomeça com um cofre vazio. Os dados que ela tinha antes não podem mais ser abertos.
reset-done-sessions = A conta foi desconectada em todos os lugares: todas as sessões dela foram encerradas.
reset-done-not-you = Se não foi você, outra pessoa pode entrar na sua conta agora. Peça um código de redefinição a um administrador do Baylee e defina de novo a sua própria senha.
reset-done-recovery-kept = Se a conta tem um código de recuperação, ele ainda abre os dados. Guarde-o: com ele, uma redefinição mantém os dados.
reset-done-recovery-new = Um código de recuperação de antes desta redefinição não abre mais nada.
password-subject = A senha da sua conta do Baylee foi alterada
password-title = A senha foi alterada
password-body = A senha da conta do Baylee deste endereço foi alterada.
password-sessions = Todas as outras sessões da conta foram desconectadas.
password-not-you = Se não foi você, outra pessoa sabe a sua senha. Defina uma nova com um código de redefinição: peça um na página de redefinição ou a um administrador do Baylee.
issued-subject = Um código de redefinição foi gerado para a sua conta do Baylee
issued-title = Um administrador gerou um código de redefinição
issued-body = Um administrador do Baylee gerou um código de redefinição para a conta do Baylee deste endereço. Com ele, é possível definir uma nova senha para a conta.
issued-valid = O código vale uma única vez, por { $hours } horas. Até ele ser usado, a sua senha continua valendo como antes.
issued-not-you = Se você não pediu, entre e altere a sua senha na página de segurança: isso encerra o código. Depois avise um administrador.

# The confirmation mail's button, which opens the security page with the code filled in.
confirm-link = Confirmar endereço
