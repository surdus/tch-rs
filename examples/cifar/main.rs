/* Training various models on the CIFAR-10 dataset.

   The dataset can be downloaded from https://www.cs.toronto.edu/~kriz/cifar.html, files
   should be placed in the data/ directory.

   The resnet model reaches XX.X% accuracy.
*/

extern crate tch;
use tch::nn::{BatchNorm2D, Conv2D, FuncT, Linear, ModuleT, SequentialT};
use tch::{nn, Device};

fn conv_bn(vs: &nn::Path, c_in: i64, c_out: i64) -> SequentialT {
    let conv2d_cfg = nn::Conv2DConfig {
        padding: 1,
        bias: false,
        ..Default::default()
    };
    SequentialT::new()
        .add(Conv2D::new(vs, c_in, c_out, 3, conv2d_cfg))
        .add(BatchNorm2D::new(vs, c_out, Default::default()))
        .add_fn(|x| x.relu())
}

fn layer<'a>(vs: &nn::Path, c_in: i64, c_out: i64) -> FuncT<'a> {
    let pre = conv_bn(&vs.sub("pre"), c_in, c_out);
    let block1 = conv_bn(&vs.sub("b1"), c_out, c_out);
    let block2 = conv_bn(&vs.sub("b2"), c_out, c_out);
    FuncT::new(move |xs, train| {
        let pre = xs.apply_t(&pre, train).max_pool2d_default(2);
        let ys = pre.apply_t(&block1, train).apply_t(&block2, train);
        pre + ys
    })
}

fn fast_resnet(vs: &nn::Path) -> SequentialT {
    SequentialT::new()
        .add(conv_bn(&vs.sub("pre"), 3, 64))
        .add(layer(&vs.sub("layer1"), 64, 128))
        .add(conv_bn(&vs.sub("inter"), 128, 256))
        .add_fn(|x| x.max_pool2d_default(2))
        .add(layer(&vs.sub("layer2"), 256, 512))
        .add_fn(|x| x.max_pool2d_default(4).flat_view())
        .add(Linear::new(&vs.sub("linear"), 512, 10))
        .add_fn(|x| x * 0.125)
}

pub fn main() {
    let m = tch::vision::cifar::load_dir(std::path::Path::new("data")).unwrap();
    let vs = nn::VarStore::new(Device::cuda_if_available());
    let net = fast_resnet(&vs.root());
    let opt = nn::Optimizer::adam(&vs, 1e-4, Default::default());
    for epoch in 1..150 {
        for (bimages, blabels) in m.train_iter(64).shuffle().to_device(vs.device()) {
            let loss = net
                .forward_t(&bimages, true)
                .cross_entropy_for_logits(&blabels);
            opt.backward_step(&loss);
        }
        let test_accuracy =
            net.batch_accuracy_for_logits(&m.test_images, &m.test_labels, vs.device(), 512);
        println!("epoch: {:4} test acc: {:5.2}%", epoch, 100. * test_accuracy,);
    }
}