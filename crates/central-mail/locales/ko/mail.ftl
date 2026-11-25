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
reset-done-subject = Baylee 계정의 비밀번호가 재설정되었어요
reset-done-title = 새 비밀번호가 설정되었어요
reset-done-body = 재설정 코드로 이 주소의 Baylee 계정에 새 비밀번호가 설정되었어요.
reset-done-kept = 계정의 데이터는 그대로예요.
reset-done-new = 계정은 빈 보관함으로 시작해요. 이전 데이터는 더 이상 열 수 없어요.
reset-done-sessions = 계정은 모든 곳에서 로그아웃되었어요. 계정의 모든 세션이 종료되었어요.
reset-done-not-you = 본인이 한 일이 아니라면 이제 다른 사람이 계정에 로그인할 수 있어요. Baylee 관리자에게 재설정 코드를 요청해서 직접 비밀번호를 다시 설정하세요.
reset-done-recovery-kept = 계정에 복구 코드가 있다면 그 코드로 데이터를 계속 열 수 있어요. 잘 보관하세요. 이 코드가 있으면 재설정해도 데이터가 남아요.
reset-done-recovery-new = 이번 재설정 이전의 복구 코드로는 더 이상 아무것도 열 수 없어요.
password-subject = Baylee 계정의 비밀번호가 변경되었어요
password-title = 비밀번호가 변경되었어요
password-body = 이 주소의 Baylee 계정의 비밀번호가 변경되었어요.
password-sessions = 계정의 다른 모든 세션이 로그아웃되었어요.
password-not-you = 본인이 한 일이 아니라면 다른 사람이 비밀번호를 알고 있어요. 재설정 코드로 새 비밀번호를 설정하세요. 재설정 페이지에서 코드를 요청하거나 Baylee 관리자에게 요청하면 돼요.
issued-subject = Baylee 계정의 재설정 코드가 만들어졌어요
issued-title = 관리자가 재설정 코드를 만들었어요
issued-body = Baylee 관리자가 이 주소의 Baylee 계정에 재설정 코드를 만들었어요. 이 코드로 계정의 새 비밀번호를 설정할 수 있어요.
issued-valid = 코드는 { $hours }시간 동안 한 번만 쓸 수 있어요. 사용하기 전까지 비밀번호는 그대로 쓸 수 있어요.
issued-not-you = 요청한 적이 없다면 로그인해서 보안 페이지에서 비밀번호를 변경하세요. 그러면 코드가 무효가 돼요. 그런 다음 관리자에게 알리세요.

# The confirmation mail's button, which opens the security page with the code filled in.
confirm-link = 주소 확인

recovery-backup-subject = Baylee 복구 코드 백업
recovery-backup-title = 복구 코드 백업
recovery-backup-intro = Baylee 계정 { $name }의 복구 코드 백업 파일이 첨부되어 있습니다.
recovery-backup-encrypted = baylee-recovery-code.zip 파일은 설정한 비밀번호로 암호화되어 있습니다.
recovery-backup-plain = baylee-recovery-code.zip 파일이 비밀번호 없이 전송되었습니다. 안전하게 보관한 뒤 이 메일을 삭제하세요.
recovery-backup-fact-file = 이 아카이브를 이 메일함과 분리된 안전한 곳에 보관하세요.
recovery-backup-fact-reset = 이 코드와 메일 재설정 모두에 접근할 수 있는 사람은 암호화된 데이터를 열 수 있습니다.
recovery-backup-warning = 본인이 요청하지 않았다면 이 메일을 무시하거나 계정 설정을 확인하세요.
recovery-backup-link = 계정 열기
invite-subject = Baylee 친구 초대
invite-title = 친구 초대
invite-intro = { $name }님이 Baylee에서 친구로 초대했습니다.
invite-note-label = 메시지:
invite-fact-friend = 이 일회용 링크를 수락하면 Baylee에서 서로 친구가 됩니다.
invite-valid-one = 초대 링크는 1일 동안 유효합니다.
invite-valid = 초대 링크는 { $days }일 동안 유효합니다.
invite-warning = 예상치 못한 초대라면 이 메일을 무시하셔도 됩니다.
invite-link = 초대 열기
