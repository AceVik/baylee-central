# The words of the mails Baylee Central sends (docs/email.md), in Traditional Chinese.
#
# One message a line: `id = text`. `{ $name }` is replaced when a mail is written.

brand = Baylee
code-label = 你的碼
code-secret = 請勿把這組碼告訴別人。Baylee 的任何人都不會向你索取它。
footer = Baylee 絕不會透過電子郵件向你索取密碼。你收到這封郵件，是因為有人為一個 Baylee 帳號填寫了這個地址。

# The code that confirms an address.
confirm-subject = 確認你在 Baylee 的電子郵件地址
confirm-title = 確認你的電子郵件地址
confirm-intro = 有人為一個 Baylee 帳號填寫了這個地址。要確認它，請在填寫地址的地方輸入這組碼。
confirm-valid = 這組碼的有效期限為 { $minutes } 分鐘。
confirm-ignore = 如果不是你本人操作的，請忽略這封郵件。沒有這組碼，這個地址不會被使用。

# The reset code. `name` is the account's sign-in name.
reset-subject = 你的 Baylee 重設碼
reset-title = 設定新密碼
reset-intro = 有人為 Baylee 帳號 { $name } 要求了一組重設碼。
reset-how = 在重設頁面輸入這組碼，即可設定新密碼。
reset-valid = 這組碼只能使用一次，有效期限為 { $minutes } 分鐘。
reset-link = 開啟重設頁面
reset-ignore = 如果不是你本人操作的，請忽略這封郵件。你的密碼維持不變。

# The notice to an address that another one replaced.
replaced-subject = 你在 Baylee 的電子郵件地址已被取代
replaced-title = 這個地址已被取代
replaced-body = 這個地址不再是某個 Baylee 帳號的電子郵件地址。另一個地址已被確認，取代了它。

# The notice to an address that was removed.
removed-subject = 你在 Baylee 的電子郵件地址已被移除
removed-title = 這個地址已被移除
removed-body = 這個地址不再是某個 Baylee 帳號的電子郵件地址。它已從該帳號中移除。

# Both notices.
notice-check = 如果不是你本人操作的，請登入並查看你帳號的安全頁面。
notice-link = 開啟安全頁面

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
