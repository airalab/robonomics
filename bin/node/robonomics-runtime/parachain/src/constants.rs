///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life>
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//
///////////////////////////////////////////////////////////////////////////////
//! A set of constant values used in substrate runtime.

/// Money matters.
pub mod currency {
    use hex_literal::hex;
    use node_primitives::{AccountId, Balance};
    use sp_core::ecdsa::Public;
    use sp_runtime::{traits::IdentifyAccount, MultiSigner};

    pub const COASE: Balance = 1_000;
    pub const GLUSHKOV: Balance = 1_000 * COASE;
    pub const XRT: Balance = 1_000 * GLUSHKOV;

    #[cfg(feature = "std")]
    lazy_static::lazy_static! {
        pub static ref STAKE_HOLDERS: Vec<(AccountId, Balance)> = sp_std::vec![
            (MultiSigner::from(Public::from_full(&hex!["4c92421bf6e871a49be4f8bddb2493010280dcc87a34a07ef4b09ca2635b2c7b7f2fdfb46b35961445a47d831d36ac2c30c823182a4f3e8f691de928bad30c1b"][..]).unwrap()).into_account(), 1847 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["daecb9a26f171b20317aeff7c44e0f52235f7cbbcdea591754c890796b981eac24317ca43c6d80e3b43665246cdf3ea155b59a983b782baa631b7e1b9df3418a"][..]).unwrap()).into_account(), 1051 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["e28957f67b2ceb54520bdec058db7aa82650030cef80c692bfd5b94de3e454faa9968aec0e3a375bea8847d4db5660e0344343427bc4670faadc81fcd9155ca8"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["113b7251ab7aba4dc499657dc549500f68ae0bb85d1c3ac5a1d45ff81b2389ff469288273240be93f13306adcf886dadc42bfd99bbe7548c02829b49685305fd"][..]).unwrap()).into_account(), 1005 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["6ea7ce4e190ef7a8848db2f3ad52bd9509349cfbf4596ba7e8bb355be9b3770695bcbdfb7606b5b710c14ecc40773f676d83cb4722fc1066c8b544d2192c885e"][..]).unwrap()).into_account(), 10562 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["27e7e0aade2514f8bffee3cd8b17d18c9c1a785be83b1b2ac6a68e5d9c2895d6d6f5a84f323c625a20372064060693c8376c2ebfae9457305de707daae624bab"][..]).unwrap()).into_account(), 19 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["4ce4fb56b970d045e19230498a458dadf9736af94ab5cca715484b1fe4c8edc15e56b5e40316820f630e767adf1a656c46afcf123c4e4805c2c88e0e8df66615"][..]).unwrap()).into_account(), 5 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["0b194207573dd6d483132a637d48bc103336794e1a052ea66656fc26bf12639ca3540f8f847f9d2e7cd77c12fe349839fc6d869cecba28e5f040793497383874"][..]).unwrap()).into_account(), 2 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["18704b1d1a43af1811bdd5c990bea0e0bdc2eb316fd65ed5f11896d3d20c77db5bf289264f9d71bdeaa995483d107a6dd61e8d07f5d8dff8b4a1879857f805ec"][..]).unwrap()).into_account(), 11 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["145ae3776b8462eb289e354581401580b34f3f93518722885baf946766b9f31cd974adcb86091f43ea3307eac27681620595819767edc833a6e02376615df20b"][..]).unwrap()).into_account(), 502 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["ef92a940e94cd7b494de2784cdfa2c55c6fd0b7b01e5589c8d7ba303e867e0512ceb5c90c54ac83f983790c0df1336e0f54a461712317c20f23543ba66b6daba"][..]).unwrap()).into_account(), 439 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["018a58b9a493ca642054285ff4670db20d4d7eb21f51c1da3e69f3f03366e9efb05d086736ed2a55f0c7ef0030ee0dc87e0b4938c690b0eba7cef13209425bf9"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["fc3d0d5e9f3727aab1eb2989fc74953d27a7e45a20dda1c996504e0db7be8e76712a1d934540cc05e36718a0bc611b4d961797445548b39462a85de429a5f366"][..]).unwrap()).into_account(), 22758 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["0645a6de403da3ed86bafc04a409a7c4daf61fea6eb55518b0458862e8c8c5fb45a0b189f3e12585a481360659f20f81940ac9724698ee36d49c55374478e543"][..]).unwrap()).into_account(), 5 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["5f272240adf3a93bfcc7ccb2a28a1a4ebc146f7d31835590fda7b066bdf8452e8fa1fdfc264548c95d05eaaf7c39a6449b76573a0f0cd478d43fd10d28a7f8bf"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["be7992ce5d40dee66f5d2efd0e25020a7f612a94805c6e6075521a393b27ba6bcbcf98396b9cbcafbb13d78f834c7d5563e5eac7cbf2d3a712e1946a3ee28f15"][..]).unwrap()).into_account(), 8009 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["fcfe849a46779c349cb56d2a5949b1bf001a4311fa0e78df00f2ad98b0468382c36d74f261365a526babb07e11d809d1b7e513d0fca6d8553cf23005823a8b70"][..]).unwrap()).into_account(), 2 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["87929c2df46a739209c6eff4dd190b902492c64cd55d69faa229884511facac3435373ebe5f772f4e6822a0dd624bf6f504f3320f0345c93594436fd1d707394"][..]).unwrap()).into_account(), 5 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["5b78fd109efcc58aaa7e48a4e9a07ac6a615b23a15d5560b2ec93642a474de4317fbf1d0bac0ffa784f60bd23576442ac1c87f8d0789def77decb6e9f59c798d"][..]).unwrap()).into_account(), 673 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["33a4d44d51d2ce8572fae15be57c915e8835c460b2b49ded425219c2744803f871e161cdf4ff0f184ac1343dbdcb5433788c96315595a5ea7583c9057488c224"][..]).unwrap()).into_account(), 2009 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["66f1ce502c9225fcba7bde31e81754d5fbb8cfe49f9db579be520bae765f57873c86adda013761713a7ff43591cabf5a364fe4d1d102e13d006b7b08ec49e49e"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["e3c85d8dded2a494b3df00b1a64dc1521667568d666cc35630b4f4cdfb922346c59f7bce0f77ae239ac683bcbb451584a827b99bc5816514f013f548fc9392dc"][..]).unwrap()).into_account(), 6 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["59bd91588c7b5e66d115ce5d2a4fd48b9e6399e4df074e2add9de46a438bdb9ebd40250f549efd986ae14b53bdc8f0ef3fad1dee3933c684d3959219e8656091"][..]).unwrap()).into_account(), 5 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["8936623b30754126646f690ac854375a750b10f5c116b39cd6bbf2d0f744787b0c9b806c04660ce06ea87d6db33ca289f2bb88314da784e2ad8bf638a71fba54"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["6d229d9515fb7adc4764a67dff68be8314d99ce5bdbf6b8123d1dae5e6e6d409e6a16d1d202de8807502e2fbcc68299547dfc93fd92edcb0703cdc28e464f863"][..]).unwrap()).into_account(), 9 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["4e3edcda30afd7087d4b4b19140eb23a09f66731dba992bb8481c34375c0927be2bd3a57c9c7199dfe110dd1f422da44bcd9f160e27dc67258bb06c8b1cd894c"][..]).unwrap()).into_account(), 10879 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["bce33ba37f5121471acb0a218e0049b0ca3e57931823754c2167d6dc9e9734cd6052eed1bb2289f8c4b1511c0ccae9d6e297c87ee074e7e39d9ca24683845c34"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["2b9163c4ccaab7de46a5a83980dbebde7a1eec6f161ca959e3e69dd56159b1180b21dbee265c72ae5d1b2e4fecdcc296f164a3b0ea7cc8a7e4775778e56fe477"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["c8758c068df9893f750680c0d4e683fa73691c1712d6cbb6d7c8664c403550690eb9d42ba87c04cf835eee224260f77cd05242eb56a99be8171d78586274a2b9"][..]).unwrap()).into_account(), 3 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["e9b52b69341e1a5a6828efe51c9dc8fba230000d2793beab4fdc8d484b719fc613f0e4a86523833b86064fac351ae6dd65b8723b14de74285312a706c6ec3c02"][..]).unwrap()).into_account(), 2656 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["fe4af50f61117b626d7f669807724b2cb7894f315e0185b967635f8e60c1ab2600913cb7a3eec9c6a10327fdb505c780929bede9b6734cf6662bafb4b34a1962"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["03773c3d6bc8a74faec0f4e035cfcb037b0d4b3a2970bbe9cc80c90e91a463a02e47b19c92fda2587993644ad199b2b93ff75edc30a12ea2cdb946ab6ffd3c80"][..]).unwrap()).into_account(), 18 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["38555ae7cbf6d048257c81dbaec2ca095677b8cb69b385b0644da41600e24a2b52825359ffc20b917c26eb2af6fbca03ba2c1ad54d5a55f79405169a07537e4c"][..]).unwrap()).into_account(), 2000 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["53fc06c8c909a3ef9931e0c560e2d2bacfa9b832d271dd05c71cba62c8d93fd45b86a27943e2522dd414bdfa527057c5839ff6483e4763b0c28c929be84194ba"][..]).unwrap()).into_account(), 100 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["765817ebf8073771a4c81d85824eb1a94f878b3d270c2674cfd286eb7fb0d6d20025c907faf73ac6ddaf1d080c2d181ef96ee8d8ab219a08155dba3c7f46278e"][..]).unwrap()).into_account(), 4 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["5449ec4c875f3f20fa74da60c31424d644031395a647a92b1d3738556c266df10fb6f34cbe2cb9265f9a57f9f95221dad28206f5094a5c4de6b7e32dfa976e05"][..]).unwrap()).into_account(), 880 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["558c1d0cbd072befdd9f5c77bfa376dfa696a199388a3110c9064a3e3961471c808646bba40a76c307236711851da05db65f38ffe7199cc8296c39d0fe7f46eb"][..]).unwrap()).into_account(), 42 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["2fef6fdf773e093e5943595cf584655f0377bb8bfc230706df2704a8bae876be992e4bdf10920fd9916e585121e76c99727bcd34b30603f98f2a8bbed64c9e73"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["9184e91e0dda8bbcd45f139f7e4761159072a6324dffc0fbc3d9c476bf3735998f9d7b79bfa1fb1f1b0412aab2c59f182dbd19484ffa40f517ea2dd503bb5e55"][..]).unwrap()).into_account(), 66 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["af636de12bb6ce76038e89b6734da6545e55da8f127ba637449f6d872b7b64790e8354927b000d0d76231304d82b7f82d0a8e1695b376b411aeeade71dfebe80"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["ae619ed9b46f5be4af379c48f4acceeafc7236989ed8741820cd3d1a357097db79a06cb1ebd04e471e4c6effafd53dbe325bf0f90e2bf7cf2ca53d213c4ff9ac"][..]).unwrap()).into_account(), 65 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["1289be53f1684f21eaf4eb15b359e445b3806522369d058f606eddaeb6f341ab91003da8c25afdab5a8b4284596e7958a7048dfc0c054667ea72061c11dcb2d2"][..]).unwrap()).into_account(), 16 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["a66a78fded5658da8af3255b4d1a0545a71c1ce2142380eabf41058f757158ac04b4ef9a6dd2cb0e15b879970cd1dfd736298d29a371cbd6321b10c85660c9be"][..]).unwrap()).into_account(), 125 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["f0601114bbeb7fc19c3e3a8fa4bf902bce7202fb37c8c96a182d091fee901ac402b7e9ae9bcbdc78cbd0208c4dd6f80686fb3872a91296d0aa93045c192d8883"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["bab43638247fd95b0066312f5f44e3826151c88adec77d06f56f2896373c82881dfc8d5e4908a21625a7bd3fea5141113498ef122e529f7a1790496e5d317275"][..]).unwrap()).into_account(), 280 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["dcde3bf097fd9a19e1ecd62dc04ba63511dd4caa3bc55bf77867e064fac2bc1ddf0f14a26de9f07f0a6bad594eedf7896dba925ddabfc317f73771c01f484d90"][..]).unwrap()).into_account(), 200 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["1988463963841d851b8a2f156f7bf8cf1d65f3ad16a43e7455f9573b7a992b95a3fb098e2d7c5aec82754a7ab6e4d0d791f087a94ba24f63063f30b900a2e782"][..]).unwrap()).into_account(), 4 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["acde5f6adefd0e035d5406bbfe156bc96ace19e63e316792a28c8f932e831cf4f7c2af768d4fa3a1e39460c582065cf0d98eaf51a28ce937a60f470e5a59fb39"][..]).unwrap()).into_account(), 96 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["393caa1ae25f2166c54618e96c777a0b4587f3fde26b8204190c8a313b64ccd40add87d2efb3ea5922ef941443a6fe550a7d1bb083efcc3b6d8865a15e3642d6"][..]).unwrap()).into_account(), 9 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["73683a24f8121641cb6da8f9804bae05940ec561d4ef7dedb8d11a315bd86dfa07292cd3d792e01e8ff1bf068a000152c024cf398a2326c6ed1436367b600c3b"][..]).unwrap()).into_account(), 2 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["89af930edac9b3949a91819ec01bb27861015c81c4a5fc3a8b358d3b2a3cd77beb2fc770c7998eaac731317d0196c87b2745b267cd3be1b1502eec2fc57268ba"][..]).unwrap()).into_account(), 7 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["2737f3ebaa7f0f6c9faa3524b480f75d7c327cd3a848aed5bbcbffabd70fd290689b40d71fc294fde564e02558b4710d349977ea04af743a161b4e20c6ce6348"][..]).unwrap()).into_account(), 56 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["ea6915920a67510e5037d016e664ca90067974fc10a9d9207b0a88262e710ad428e2c0ea7d236360335df2d76ab1279c35c451f5decd55ea0af227f711725dfb"][..]).unwrap()).into_account(), 8 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["8cd8f20f8862f3b68a9d6988a11799a08d9cf02102850b89c9863db74d2554d24d0236475ec00888b6554b05f78287d0982e2e269076c7ef97c34a0b7da04782"][..]).unwrap()).into_account(), 4623 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["f461f9a34c185cf9d76090264f12a0ab409effcb59c02961bbf35594940a89430c64a811d5ea13a554fbf5c125055e4837ca0d609875d104255251e2ac838ac8"][..]).unwrap()).into_account(), 40 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["f39fabbad1c1612c931ca1b2245ceda6f74b40e2401214b3b9ec72f525be668de37eb02b05131da4cc1a925e566a165dc0efedf8a8aab0fc64e6b7da6af604b2"][..]).unwrap()).into_account(), 45 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["71ed0c305dfaabe431c85c7b33fb74b8effd956b088682ca991443110bcc839c6b1f87129549c3c43a16d391e84e21f2c6b240585481fe48f2f61d3e0b2c7213"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["8984b8746a06b6378c924ac6d3f412351418a5b7e9794aca8218c681e653c8f43e8944d130677886774a7ff383dbfc7e80bd6ef06d388817f65b77b5b05e4315"][..]).unwrap()).into_account(), 120 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["b1d5ad3026220a2f569b3f2f778bba6fb3185b90f649c183ac445178a29b7ac391b4ed70fd2fa4e44de09194cd1efa4fa093269ba2307fb877220cc3e5784065"][..]).unwrap()).into_account(), 6 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["5c745b945b246c7b269de1fab38a9a0c31bf6fc334cc36a126dc4a01a0b5515a70fc07dc14986f9710f2115d8de6fbefb9d9a7d7262bb4c6921ed59073b6848f"][..]).unwrap()).into_account(), 120 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["200d334a6cb626cd02efd39e7479331786e8ee70b9cf332966ed886081c20b4e083154cdee16c43e90cb74681e6ca94e2d71477b8836c67d6093b3e10b5a20c9"][..]).unwrap()).into_account(), 671 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["07ed0b91b20d39b9f20876f92db6a0d5993ae7d5fcd23d5fa3d6d22e3bd58a5a751026babcc9dac4d33e5a6b458cd813d0cb7da01699dbc8f60ed4fd4f6d4f72"][..]).unwrap()).into_account(), 26 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["0cc4ed21767b2a3ddfe6fdad5bccd51d9a4aa0d330e1f556f108176cfba1184a1dc408b06a03053d17a8ccf3f669bcbf1e5bc2e20e087e9780a4072a0725dbb5"][..]).unwrap()).into_account(), 441 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["9e600278162aa931d29f574d5dc613545b27c72c60ab4016099b134fbe1fafd4c1b7c4d37036c8c49faae57bc0e616e3bcd2b27d89821560b337d005371e7722"][..]).unwrap()).into_account(), 21 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["17c617e92cf8b54725f59d4c7410f44d0b7f3eaf47bd34c9d92c9e4bb406e818cd7ea517d802898eec99b1d50cfb9329bc7c5b9356918e18cbdc542a8caaeb4c"][..]).unwrap()).into_account(), 171 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["ccb6a58c290f564f94d189d06801767e84b0549d23b41f688653f75e513e6d1af6d1a516f6bcd17a2797f9a31e8cb9b4f0d2abadbd834751fb098fbc791fa101"][..]).unwrap()).into_account(), 180 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["c4f6ab1c9bae07941941d57bfdf372c32342c17d84d0fbfe990000d6803c5d2531a46d2a7f67b13fc74e058951567275c246f9fd1ddf265306dc48711ab19ab9"][..]).unwrap()).into_account(), 1219 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["67326443018b63c365b3872af749cee29c12c973accb4e9cca73232228e542f26da82e2da8bcd5d76c06d498053a70fa83030bca1628e36c6b6ab60e23847d6e"][..]).unwrap()).into_account(), 80 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["9b80cd4aecfa8ac71370a43fb8bd124164c9f18b9d441a6d32b475927d402dacfcee9a3eed5e541e95bdabd83420aa26367be3e47ae94ca24895ba58fdc15f73"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["f2bd3ace215597523784a17fa723b9c92c7be2e70b069d71b9628a8853a9abea48a7cfe41c68d5cc2bcbb0e2108199fc60697910367cb05fd577335993ca91e8"][..]).unwrap()).into_account(), 30 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["11a26fb27bf61b14c6b83412bc247535be6ed3097e1d43a9afca0bd5c7fb18a5c83b301b39f661bb79d491c447c35eafaa4e4941de099b8f49cbc310c29b0336"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["1a24e9988cf5f5ce7bd53e65146114e730cf94d5199095aec069e2d97c654dea841714d86b167abf3b63c6dad1da4382d056fc5b6800357bd8ee7e43c638508b"][..]).unwrap()).into_account(), 21 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["7ff32a62dcae61d7b2f80dddda6d1f117a4aefa52d63dfd0184ac196caef861e704a608ab583123af66187d0ef878e510a79a3fc34e217f80feda2a531a878f6"][..]).unwrap()).into_account(), 219 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["85cd4199e77b14980cc4b15eb9f3ee4462507edb3bf4d71d76cd15a7405fa7fc59d2992384c6ff167f6dae5593502635ddd88464045106fe09484a34a97f74a5"][..]).unwrap()).into_account(), 599 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["22d02848c89b097c7472e92b88ae1c498bec2cb220c34a95f7c6ab0af1c970cb9fc498305af4cd267bd612220b118199a97d42b95435a867706a190bad071b61"][..]).unwrap()).into_account(), 241 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["fa733c6bfde07b0996b87a6727d9ed74612b8d02d4944171cfc0eb34e08604442fa7b94981b40923602bf0e9e13de50fd4ceac89d1d3268f2b12f511ef260469"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["0b0449704f23045b2f6b3aa8f7746eb82155e0c68257d689dff01c94306ec61b1693f067c6680d0a273b1ff33f0394a7f34e2c1b98f2bb0f5441cdffc55033f2"][..]).unwrap()).into_account(), 110 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["68430ded8c9b2cbf8b83d1e26b817141a7393f9f82f03aab19743ebb12bbf281810a1ce9da2271ee739b1d242e233dd90edf7c9c14d24c7f15310420db4af787"][..]).unwrap()).into_account(), 359 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["b1dbc50a6259bbc8a661ef7a558a34798c4bb9b1992a1b9a3b169058836da00d637cf4d40560d9e6601df9644497b6714f67b6f5f03034f252c05b427fb6edb2"][..]).unwrap()).into_account(), 47 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["32bb455a60bc5e8bc4fd21e739af8bafa04f5a8c88e86ed4292d0855309d6d83a64ce5672d5a36abd29ff529504dc99b5793cfd9be202752808796ee837e6d79"][..]).unwrap()).into_account(), 291 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["48e819e158fcb023487aa7851429dc4280f13650aacd6a60a61ebefcecb57a66ea43823296691794358de2419abddbfc4db04c5ac47e53482689fe685476b61c"][..]).unwrap()).into_account(), 760 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["b2dc8585c6aaa4693b94f017e9d9ed3346258a034d75d97a91d5babc5c0da83a47afc8cbcc122f8ce11206d67505ff50b2b3180c6c7f91fb8e2eedcc5f0d85e9"][..]).unwrap()).into_account(), 7 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["1e5edcab072a96ab4300293461bb82481dffb744765457916982f194c60d76391d1c212582cb183bbaf11a34961c60b465d40441076fa1f770ab35d20aba485e"][..]).unwrap()).into_account(), 200 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["6ad01db6a8016f2b1e887c5c4b9aa29309b81337fc6830ef8034e48e78afe4420c41e95d43eb9a766989bb046a0bfd99dccd6e26bcb365638ae8d919ad85d6ff"][..]).unwrap()).into_account(), 54 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["9b93691c046bbe886dfc47d43c9158bf3648063324f9dd6eb3656bcd4c46111d4cc43267ae674e5308bd86773f48786a2b1ac0e74d6d7f4409607633a5cdf48c"][..]).unwrap()).into_account(), 617 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["bcc7e6ed9a58c8e9bca1f993513a7d1158bb1d51e2cfb7e6860fd46daaf4ce9427f2a3558b5a13f1a23e673f4e2797e541fd89477cf6528c22989c1c47c4ce82"][..]).unwrap()).into_account(), 284 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["340356e84d4c09a74ceb4bb72774b12d573971ef760a91984cebed114496f659d70fde5955695891229701854a8e64e4b73ebf6c7670400368943041ef167c37"][..]).unwrap()).into_account(), 82 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["23f395528d4b3466e083f6ea529fccd6a9165c7a017142de4abf00c53a121884cc3a48eaac556b03e97d5dfaf4b4241acd1d5e8bd2f9a891fa923b53705c10e4"][..]).unwrap()).into_account(), 2 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["7de7498041403dab9077927196d55763943049c4ad4886c72e741b66693755aef7a858cc947884c10f81bea0aaced97fb46f45bc69f8ea786dc98ee2704d9596"][..]).unwrap()).into_account(), 129 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["13c8ce126ee46fba49f3d6ad6e5b368da6460301bd26e0beff0fff2a6f711ed19fc0e54295468029500486b697985700bc1ae28bf2300e0a497f67e7fea65f1d"][..]).unwrap()).into_account(), 37 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["21824eafba9179d1c15ac06486cfa21eae55dd16ff952fed8e6e55bd1fcab2e90b28bc3ec7788fa64f507177110b37508c116c240f9c0037075522185914b616"][..]).unwrap()).into_account(), 161 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["b7867de887ca7b9a732f87fe58bc5c68be6e810604332610ccc2687a6eb00da9782c5336f95a40f5392775b625b1e8c3ea32af130c50a97e11206d9305aef6e0"][..]).unwrap()).into_account(), 71 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["c7d0728a795f1c1b2337e0734d8c16bac3e10c751da8686e29b3426a55a5bee0ca834bf6b6bc76c892658ad3a06205efd6680fb87dfedcd2aae829a82ebf13f0"][..]).unwrap()).into_account(), 79 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["9e791846756bef3d8cd29a67fd31f57a12c5eb2f5be7f8c2946f1cab7ea519755fb67d0c607f709faf7c4865b9fb3c32c679a3011516e050ed394b88bdd88f13"][..]).unwrap()).into_account(), 220 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["f1e1bfdd0cfe8dea40c1efe4f7e89827ae2ad2c1be063364d9ee470eda37df0e5595158c985846afb738687b575eaa440c5fee71e9f95763c07304fd9d77c48d"][..]).unwrap()).into_account(), 191 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["0287d8b28dea13e375b847f23ce90240efd65d7ebd604137221b3a39cc2e42eaf96c77194f9881ec78558d0d61b11e1c37456023809103af4fddaf1379ed1148"][..]).unwrap()).into_account(), 40 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["f84b188fdd58c2b717b0e5ed0b75163ed2b1330656d05113ad0e279ca9d83813816f3718770cda35fe0c74e877fcdd98ae794f5f264ba13421d0034be0428d41"][..]).unwrap()).into_account(), 2271 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["d1939f2fce818cdeb62a31600c8cb2ab010260e36a5b0189a9293b91b22bd13588010cc1f4473882c9ef36c509d7bf6faea6d597bddc6ff53c8a5d8e29c61ae5"][..]).unwrap()).into_account(), 13 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["ac6a6689f15d4be575a59b56a360eb24012e1f46abfed644b0cbb95a7da9e62ae93974d71d6451c295580d8b12c6c3df298bc04f8e957d7b9f9cc46849c03ee8"][..]).unwrap()).into_account(), 50 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["d56c8b20c767b4eb81add1be713fa21eb3746d78a9c29fe720fb8d89646e15e76315fb05752d8c0d3b16a1daf080bd325ffda95e0a01fc4b7137d1bfc579d7e9"][..]).unwrap()).into_account(), 123 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["87809308bddcfe0470bbf92e0b2e99e950eae44027890481f338434dc2e230851b83ac6fe74dd3189b30175ad8941792717ef77209ec9d009b8fe71d7e80d25d"][..]).unwrap()).into_account(), 80 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["8a7a2bccf09b4c645fa83dca39e02ba6e1f41efd6cb97e9c912fcc57475c2f10faec7a7218e68c7b3a0d2a383a09996f39e574be53a7d1b1a88d56c5697d0d5d"][..]).unwrap()).into_account(), 9 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["9a3db8697c149bc5613c3d0484261b849e26c94f8239490837497373c0ec1254f1c9fcd9fd199656a83d10287f7b1b6064c3b0c5dd1433255909113236ed55f1"][..]).unwrap()).into_account(), 51 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["1ded97505a419d73f917b5380159050288550c42e474e75278a1ceeb05cdddbec7739eb43943639c533650b7c562518fa1f6db970ce3e23cfed6fdfd080c457f"][..]).unwrap()).into_account(), 3 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["73c58145b6e4329fc8a91fd21b0d4a5342e24b7dffd0e6d8d02134d57ff3cdaa1545b8452d9049bc92e93750a62cbcb7eecf883372392b344428fe858e6d9080"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["49dd7043df24b716e46ed39b72963c479881bc6914e3eaa76f00ddae993b1c73837bee8ff538cb6d563a5cb51cc86643d4272e6967a7ebf4aeb232ff6d0aed09"][..]).unwrap()).into_account(), 10 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["7c51944c3d854ffeea89fbcbcc220876659cdb62b121cfe4be7eda4be6d9dcaf824465e6180aff7d4053afebc3b279411dd6ce82fab40891dda440daaee29682"][..]).unwrap()).into_account(), 1 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["bf800934225629c7b60ccf3c727fee1da53201a0314eb488d044850ebab76fc5e54d76f13544bb085628f07cf8c8fe57f2adc6991ed6f06fa1645566fa9ca9f7"][..]).unwrap()).into_account(), 8 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["f42f3f3dde61cb819406ed2d2a9bbc58582c22d6df4f6521288a5727d6086dbcb41e62095e6f399ec5c3c9605db35fbd0b2e2116d2843a8533215e02617379fb"][..]).unwrap()).into_account(), 11 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["06e2c3458f72bebc0641eb8810da53a78738686e653bc74ab4dd6a576e402b1132b341bf8a7e4faf1185e58710676cdb31d658ec264099db9092a81fd595650c"][..]).unwrap()).into_account(), 2773 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["21896ea4aa4903cc674d2f2dd717696f6c641becbcab37ec3d842bb95c130fbba457647351c00516710be2d46bbfa178ff4d374efd581d110914ebb9b6bcdcdd"][..]).unwrap()).into_account(), 21 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["8d1210810a74c98f214f6ca836e4bbea2bb223c2e5e4f53ee092d53cd988901ce3e5991b7b5d53819eaf05e4c07fcf6f2ec7f24ac710360c5975cc45ef800bb0"][..]).unwrap()).into_account(), 6985 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["a6656746a3321fb09e7a8577f6007d29d3144705bb6f263a2c88ea382189c0e8e71b4a2c8f22cf69a2cf145060baff83e744a1140d83d441ba0b6f9a00b1d7ba"][..]).unwrap()).into_account(), 10 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["41d45ffa33b1897f97bce7c670f030f5683ad907874b50aa5fb53ac0874192e053b12fc3e08fe3c7353ee476b51a58bb3b9e06150e4758fd1e31777bdb46e157"][..]).unwrap()).into_account(), 405 * XRT),
            (MultiSigner::from(Public::from_full(&hex!["adcd2e476f6d3d2a3f159923c4010794f3f0bc5c02828b78a23409daa2032545da87d059d757300e5e8872baf834d94803f8e1a6e11dac48937dce47066ba2f4"][..]).unwrap()).into_account(), 26 * XRT),
        ];
    }
}

/// Time.
pub mod time {
    use node_primitives::{BlockNumber, Moment};
    pub const MILLISECS_PER_BLOCK: Moment = 6000;
    pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;
    pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
}

// CRITICAL NOTE: The system module maintains two constants: a _maximum_ block weight and a
// _ratio_ of it yielding the portion which is accessible to normal transactions (reserving the rest
// for operational ones). `TARGET_BLOCK_FULLNESS` is entirely independent and the system module is
// not aware of if, nor should it care about it. This constant simply denotes on which ratio of the
// _maximum_ block weight we tweak the fees. It does NOT care about the type of the dispatch.
//
// For the system to be configured in a sane way, `TARGET_BLOCK_FULLNESS` should always be less than
// the ratio that `system` module uses to find normal transaction quota.
/// Fee-related.
pub mod fee {
    pub use sp_runtime::Perbill;

    /// The block saturation level. Fees will be updates based on this value.
    pub const TARGET_BLOCK_FULLNESS: Perbill = Perbill::from_percent(25);
}
