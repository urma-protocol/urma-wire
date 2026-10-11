# Licenses and asset provenance

Project-authored application source is licensed under BSD Zero Clause (0BSD),
as provided in [LICENSE](LICENSE). Third-party dependencies keep their own
licenses; the application license does not replace those licenses or grant
trademark rights.

## Bundled visual assets

Primary upstream sources and licenses were checked on 11 October 2026. Both coin
icons are published upstream under **CC0-1.0**. The shared legal text is reproduced
below; neither CC0 nor the application license grants trademark rights.

- `assets/litecoin.svg` is byte-identical to `svg/color/ltc.svg` from
  [spothq/cryptocurrency-icons at `1a63530be6e374711a8554f31b17e4cb92c25fa5`](https://github.com/spothq/cryptocurrency-icons/blob/1a63530be6e374711a8554f31b17e4cb92c25fa5/svg/color/ltc.svg).
  The icon project credits Christopher Downer and its contributors.
  Size: 331 bytes; SHA-256:
  `21b1ffe201826ba5d027f814baab9103938898c0be374d3cc948bd119b55ce66`.
  [Upstream LICENSE.md](https://github.com/spothq/cryptocurrency-icons/blob/1a63530be6e374711a8554f31b17e4cb92c25fa5/LICENSE.md)
  SHA-256: `a2010f343487d3f7618affe54f789f5487602331c0a8d03f49e9a7c547cf0499`.
- `assets/bitcoin.svg` comes from
  [BitPay bitcoin-brand at `887f040f7c44660b84c4000f4a54897ba5518194`](https://github.com/bitpay/bitcoin-brand/blob/887f040f7c44660b84c4000f4a54897ba5518194/bitcoin.svg).
  The only difference is CRLF-to-LF line endings. Upstream: 1452 bytes,
  SHA-256 `3672ebaa9a2c6e7490636c8f3fc093ed1601f9aca103f164a6926cd1a5488d77`;
  local: 1437 bytes,
  SHA-256 `5083e5447d66c4654977a891c79d3abd3fb03c2a5e6b759458c70147cd48bd9f`.
  [Upstream LICENSE](https://github.com/bitpay/bitcoin-brand/blob/887f040f7c44660b84c4000f4a54897ba5518194/LICENSE)
  SHA-256: `6d489af6292662d9e36d34ce49423784984a5f6e41d7b58f49b01264df59fa03`.
  Its [README license statement](https://github.com/bitpay/bitcoin-brand/blob/887f040f7c44660b84c4000f4a54897ba5518194/README.md#license)
  identifies BitPay, Inc. as the CC0 dedicator.

The two upstream license texts are identical except for final whitespace. No
coin asset bytes were changed during this verification.

`assets/circuit-eagle.svg` and `assets/urma-wire.svg` are the existing ROSINT eagle
and URMA Wire artwork. `assets/urma-wire-icon.png` and `src-tauri/icons/` contain
the static Wire icon and generated platform sizes. Application source licensing
alone is not an explicit branding grant. Record the rights holder's redistribution
permission before public distribution; regenerating sizes does not establish
rights to the source artwork. This does not block local operator inspection.

## Public test identities

`app-engine/tests/fixtures/valid.vault` is intentionally committed public test data.
Its public password is used by `scripts/browser_fixture_test.py`, and
`app-engine/tests/wallet.rs` supplies its public recovery phrase. Other test modules
create disposable identities with explicit fixture passwords. They are not user
vaults or production credentials. Anyone can control those keys: never fund them
with valuable coins or import them as a personal identity.

The optional `fixture-harness` frontend feature exposes test commands. It must
remain disabled for operator use and ordinary distributed builds. Runtime vaults,
plans, build outputs and browser profiles are not release inputs.

## CC0 1.0 Universal — coin icon license

```text
Creative Commons Legal Code

CC0 1.0 Universal

    CREATIVE COMMONS CORPORATION IS NOT A LAW FIRM AND DOES NOT PROVIDE
    LEGAL SERVICES. DISTRIBUTION OF THIS DOCUMENT DOES NOT CREATE AN
    ATTORNEY-CLIENT RELATIONSHIP. CREATIVE COMMONS PROVIDES THIS
    INFORMATION ON AN "AS-IS" BASIS. CREATIVE COMMONS MAKES NO WARRANTIES
    REGARDING THE USE OF THIS DOCUMENT OR THE INFORMATION OR WORKS
    PROVIDED HEREUNDER, AND DISCLAIMS LIABILITY FOR DAMAGES RESULTING FROM
    THE USE OF THIS DOCUMENT OR THE INFORMATION OR WORKS PROVIDED
    HEREUNDER.

Statement of Purpose

The laws of most jurisdictions throughout the world automatically confer
exclusive Copyright and Related Rights (defined below) upon the creator
and subsequent owner(s) (each and all, an "owner") of an original work of
authorship and/or a database (each, a "Work").

Certain owners wish to permanently relinquish those rights to a Work for
the purpose of contributing to a commons of creative, cultural and
scientific works ("Commons") that the public can reliably and without fear
of later claims of infringement build upon, modify, incorporate in other
works, reuse and redistribute as freely as possible in any form whatsoever
and for any purposes, including without limitation commercial purposes.
These owners may contribute to the Commons to promote the ideal of a free
culture and the further production of creative, cultural and scientific
works, or to gain reputation or greater distribution for their Work in
part through the use and efforts of others.

For these and/or other purposes and motivations, and without any
expectation of additional consideration or compensation, the person
associating CC0 with a Work (the "Affirmer"), to the extent that he or she
is an owner of Copyright and Related Rights in the Work, voluntarily
elects to apply CC0 to the Work and publicly distribute the Work under its
terms, with knowledge of his or her Copyright and Related Rights in the
Work and the meaning and intended legal effect of CC0 on those rights.

1. Copyright and Related Rights. A Work made available under CC0 may be
protected by copyright and related or neighboring rights ("Copyright and
Related Rights"). Copyright and Related Rights include, but are not
limited to, the following:

  i. the right to reproduce, adapt, distribute, perform, display,
     communicate, and translate a Work;
 ii. moral rights retained by the original author(s) and/or performer(s);
iii. publicity and privacy rights pertaining to a person's image or
     likeness depicted in a Work;
 iv. rights protecting against unfair competition in regards to a Work,
     subject to the limitations in paragraph 4(a), below;
  v. rights protecting the extraction, dissemination, use and reuse of data
     in a Work;
 vi. database rights (such as those arising under Directive 96/9/EC of the
     European Parliament and of the Council of 11 March 1996 on the legal
     protection of databases, and under any national implementation
     thereof, including any amended or successor version of such
     directive); and
vii. other similar, equivalent or corresponding rights throughout the
     world based on applicable law or treaty, and any national
     implementations thereof.

2. Waiver. To the greatest extent permitted by, but not in contravention
of, applicable law, Affirmer hereby overtly, fully, permanently,
irrevocably and unconditionally waives, abandons, and surrenders all of
Affirmer's Copyright and Related Rights and associated claims and causes
of action, whether now known or unknown (including existing as well as
future claims and causes of action), in the Work (i) in all territories
worldwide, (ii) for the maximum duration provided by applicable law or
treaty (including future time extensions), (iii) in any current or future
medium and for any number of copies, and (iv) for any purpose whatsoever,
including without limitation commercial, advertising or promotional
purposes (the "Waiver"). Affirmer makes the Waiver for the benefit of each
member of the public at large and to the detriment of Affirmer's heirs and
successors, fully intending that such Waiver shall not be subject to
revocation, rescission, cancellation, termination, or any other legal or
equitable action to disrupt the quiet enjoyment of the Work by the public
as contemplated by Affirmer's express Statement of Purpose.

3. Public License Fallback. Should any part of the Waiver for any reason
be judged legally invalid or ineffective under applicable law, then the
Waiver shall be preserved to the maximum extent permitted taking into
account Affirmer's express Statement of Purpose. In addition, to the
extent the Waiver is so judged Affirmer hereby grants to each affected
person a royalty-free, non transferable, non sublicensable, non exclusive,
irrevocable and unconditional license to exercise Affirmer's Copyright and
Related Rights in the Work (i) in all territories worldwide, (ii) for the
maximum duration provided by applicable law or treaty (including future
time extensions), (iii) in any current or future medium and for any number
of copies, and (iv) for any purpose whatsoever, including without
limitation commercial, advertising or promotional purposes (the
"License"). The License shall be deemed effective as of the date CC0 was
applied by Affirmer to the Work. Should any part of the License for any
reason be judged legally invalid or ineffective under applicable law, such
partial invalidity or ineffectiveness shall not invalidate the remainder
of the License, and in such case Affirmer hereby affirms that he or she
will not (i) exercise any of his or her remaining Copyright and Related
Rights in the Work or (ii) assert any associated claims and causes of
action with respect to the Work, in either case contrary to Affirmer's
express Statement of Purpose.

4. Limitations and Disclaimers.

 a. No trademark or patent rights held by Affirmer are waived, abandoned,
    surrendered, licensed or otherwise affected by this document.
 b. Affirmer offers the Work as-is and makes no representations or
    warranties of any kind concerning the Work, express, implied,
    statutory or otherwise, including without limitation warranties of
    title, merchantability, fitness for a particular purpose, non
    infringement, or the absence of latent or other defects, accuracy, or
    the present or absence of errors, whether or not discoverable, all to
    the greatest extent permissible under applicable law.
 c. Affirmer disclaims responsibility for clearing rights of other persons
    that may apply to the Work or any use thereof, including without
    limitation any person's Copyright and Related Rights in the Work.
    Further, Affirmer disclaims responsibility for obtaining any necessary
    consents, permissions or other rights required for any use of the
    Work.
 d. Affirmer understands and acknowledges that Creative Commons is not a
    party to this document and has no duty or obligation with respect to
    this CC0 or use of the Work.
```
