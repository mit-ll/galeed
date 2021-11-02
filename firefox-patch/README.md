## Overview

To obtain the firefox source code, first install git-cinnabar:

https://github.com/glandium/git-cinnabar

Next, use git to clone the firefox source code:

```$ git clone hg::https://hg.mozilla.org/mozilla-unified ```

Check out commit 691b7fa1de4ae8f4600f4efb823444a03cd38ec0

```$ git checkout 691b7fa1de4ae8f4600f4efb823444a03cd38ec0 ```

Apply our patch:

```$ git apply galeed_patch.patch ```

## Disclaimer

Galeed is distributed under the terms of the MIT License

DISTRIBUTION STATEMENT A. Approved for public release: distribution unlimited.

© 2021 MASSACHUSETTS INSTITUTE OF TECHNOLOGY

    Subject to FAR 52.227-11 – Patent Rights – Ownership by the Contractor (May 2014)
    SPDX-License-Identifier: MIT

This material is based upon work supported by the Under Secretary of Defense (USD) for Research & Engineering (R&E) under Air Force Contract No. FA8702-15-D-0001. Any opinions, findings, conclusions or recommendations expressed in this material are those of the author(s) and do not necessarily reflect the views of USD (R&E).

The software/firmware is provided to you on an As-Is basis
