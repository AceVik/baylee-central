# The words of the mails Baylee Central sends (docs/email.md), in Korean.
#
# One message a line: `id = text`. `{ $name }` is replaced when a mail is written.

brand = Baylee
code-label = 내 코드
code-secret = 이 코드를 다른 사람에게 알려 주지 마세요. Baylee에서는 누구도 이 코드를 묻지 않아요.
footer = Baylee는 이메일로 비밀번호를 묻지 않아요. 이 주소가 Baylee 계정에 입력되어서 이 메일을 받으셨어요.

# The code that confirms an address.
confirm-subject = Baylee 이메일 주소 확인
confirm-title = 이메일 주소 확인
confirm-intro = 이 주소가 Baylee 계정에 입력되었어요. 주소를 확인하려면 주소를 입력한 곳에 이 코드를 입력하세요.
confirm-valid = 이 코드는 { $minutes }분 동안 유효해요.
confirm-ignore = 본인이 한 일이 아니라면 이 메일을 무시하세요. 코드 없이는 이 주소가 사용되지 않아요.

# The reset code. `name` is the account's sign-in name.
reset-subject = Baylee 재설정 코드
reset-title = 새 비밀번호 설정
reset-intro = Baylee 계정 { $name }의 재설정 코드가 요청되었어요.
reset-how = 재설정 페이지에 이 코드를 입력하면 새 비밀번호를 설정할 수 있어요.
reset-valid = 이 코드는 한 번만, { $minutes }분 동안 쓸 수 있어요.
reset-link = 재설정 페이지 열기
reset-ignore = 본인이 한 일이 아니라면 이 메일을 무시하세요. 비밀번호는 그대로예요.

# The notice to an address that another one replaced.
replaced-subject = Baylee의 이메일 주소가 바뀌었어요
replaced-title = 이 주소는 다른 주소로 바뀌었어요
replaced-body = 이 주소는 더 이상 Baylee 계정의 이메일 주소가 아니에요. 그 자리에 다른 주소가 확인되었어요.

# The notice to an address that was removed.
removed-subject = Baylee의 이메일 주소가 삭제되었어요
removed-title = 이 주소는 삭제되었어요
removed-body = 이 주소는 더 이상 Baylee 계정의 이메일 주소가 아니에요. 계정에서 삭제되었어요.

# Both notices.
notice-check = 본인이 한 일이 아니라면 로그인해서 계정의 보안 페이지를 확인하세요.
notice-link = 보안 페이지 열기

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
