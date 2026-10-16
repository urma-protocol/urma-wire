"""Profile editor checks in the existing disposable browser fixture gate."""
import time


def profile_editor_fixture(js):
    js("history.pushState({},'', '/identity');window.dispatchEvent(new PopStateEvent('popstate'))")
    for _ in range(100):
        if js("return !!document.querySelector('textarea')"):
            break
        time.sleep(.05)
    else:
        raise AssertionError("Profile editor did not mount")
    js("[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Refresh session status').click()")

    def snapshot():
        return js("""
            const t=document.querySelector('textarea');
            const b=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Review nickname & fee');
            return {text:t.value,counter:t.closest('label').querySelector('.field-hint').textContent,
                warning:t.closest('label').querySelector('[role=alert]').textContent,
                disabled:b.disabled,maxlength:t.hasAttribute('maxlength')};
        """)

    for _ in range(100):
        if not snapshot()["disabled"]:
            break
        time.sleep(.05)
    else:
        raise AssertionError("Unlocked profile editor review remained disabled")
    for text in ["x" * 128, "x" * 129, "😀" * 32, "😀" * 33, "😀" * 32]:
        count = len(text.encode("utf-8"))
        js("const t=document.querySelector('textarea');t.value=arguments[0];t.dispatchEvent(new Event('input',{bubbles:true}))", text)
        for _ in range(100):
            state = snapshot()
            if (f"{count} / 128 UTF-8 bytes." in state["counter"]
                    and state["disabled"] == (count > 128)):
                break
            time.sleep(.02)
        else:
            raise AssertionError(state)
        assert state["text"] == text, state
        assert not state["maxlength"], state
        assert ("Nickname exceeds" in state["warning"]) == (count > 128), state
    print("PASS: real browser nickname textarea/counter/review enabled128-disabled129; emoji32=128/33=132/re-enabled128; no maxlength or truncation")
