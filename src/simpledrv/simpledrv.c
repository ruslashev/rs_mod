#define pr_fmt(fmt) "%s:%s: " fmt, KBUILD_MODNAME, __func__

#include <linux/device.h>
#include <linux/fs.h>
#include <linux/init.h>
#include <linux/kdev_t.h>
#include <linux/module.h>
#include <linux/types.h>
#include <linux/uaccess.h>
#include <linux/version.h>

static const char* drv_name = "simpledrv";

static const char   strdata[] = "Hello from kernel!\n";
static const size_t lendata   = sizeof(strdata);

static dev_t maj = 0;
static struct class *dev_class = NULL;
static struct device *cdev;
static int dev_num;

static struct file_operations fops;

static int __init sdrv_init(void)
{
	pr_info("init, registering device...\n");

	maj = register_chrdev(0, drv_name, &fops);

	if (maj < 0) {
		pr_err("failed to register chardev: res=%d\n", maj);
		return maj;
	}

	pr_info("registered chardev maj=%d\n", maj);

#if LINUX_VERSION_CODE >= KERNEL_VERSION(6, 4, 0)
	dev_class = class_create(DRV_NAME);
#else
	dev_class = class_create(THIS_MODULE, drv_name);
#endif

	if (IS_ERR(dev_class)) {
		pr_err("failed to create device class: err=%ld\n", PTR_ERR(dev_class));
		unregister_chrdev(maj, drv_name);
		return PTR_ERR(dev_class);
	}

	pr_info("created device class\n");

	dev_num = MKDEV(maj, 0);
	cdev = device_create(dev_class, NULL, dev_num, NULL, drv_name);

	if (IS_ERR(cdev)) {
		pr_err("failed to create device: err=%ld\n", PTR_ERR(cdev));
		class_destroy(dev_class);
		unregister_chrdev(maj, drv_name);
		return PTR_ERR(cdev);
	}

	pr_info("created device\n");

	return 0;
}

static ssize_t sdrv_read(
		struct file* file_ptr,
		char __user* user_buffer,
		size_t count,
		loff_t* offset)
{
	pr_info("read offset=%llu, bytes=%zu\n", *offset, count);

	if (*offset >= lendata)
		return 0;

	if (*offset + count >= lendata)
		count = lendata - *offset - 1;

	if (copy_to_user(user_buffer, strdata + *offset, count) != 0)
		return -EFAULT;

	*offset += count;

	return count;
}

static void __exit sdrv_exit(void)
{
	pr_info("exit, unregistering device...\n");

	if (!IS_ERR(dev_class))
		device_destroy(dev_class, dev_num);

	if (!IS_ERR(dev_class) && !IS_ERR(cdev))
		class_destroy(dev_class);

	if (maj != 0)
		unregister_chrdev(maj, drv_name);
}

static struct file_operations fops = {
	.owner = THIS_MODULE,
	.read  = sdrv_read,
};

module_init(sdrv_init);
module_exit(sdrv_exit);

MODULE_AUTHOR("Ruslan Akbashev");
MODULE_DESCRIPTION("Simple open/read/close driver");
MODULE_LICENSE("GPL");

