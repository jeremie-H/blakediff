# blakediff

_⚠ This repository is only for testing purpose. It shouldn't be used on production._

A simple first version to make hash on all files in a directory and generate a report file with this format :

```shell
<hash_1> <path_file_1>
<hash_2> <path_file_2>
```

## how to build and install ? i want to use it quickly !
ok let's go
```
cargo install --path .
```
## yeah ok, pretty simple... but how will I update the binary with new features in the future ? 🤔
```
git pull
cargo install --path .
```
_PS : this is the same command above ⬆️😎_


## Command `generate`
Use this first subcommand to generate an output with all hashes and path's files and redirect this output into a report file.
```shell
blakediff generate <path_directory>  > report_file_1
```
the option `--parallel` or `-p` can be used to walk directories tree in multithreading (should be used only on ssd).  


_Exemples :_
```
blakediff generate ~/Music > ~/hashmusics_local.txt
blakediff generate /mnt/smbmount/Music > ~/hashmusics_smb.txt
```

## Command `compare`
Generate all missing hash from the path1 in path2 and missing file from the path2 in path1
```shell
blakediff compare <report_file_1> <report_file_2>
```

_Exemples :_
```
blakediff compare ~/musiques_hash_local.txt ~/musiques_hash_smb.txt
```

_Results exemples:_
```
only in ~/musiques_hash_smb.txt : /home/jeremie/Music/my_song012.mp3
only in ~/musiques_hash_smb.txt : /home/jeremie/Music/my_song234.mp3
only in ~/musiques_hash_local.txt : /home/jeremie/Music/my_song456.mp3
duplicates : /home/jeremie/Music/shame.mp3 /mnt/smbmount/shame.mp3
```


## Perf comparison with sha256sum
In order to avoid device bottleneck, put the directory to hash in a tmpfs, exemple with this 10G ramdisk :
```
sudo mount -t tmpfs -o size=10G tmpfs /media/virtuelram
```

- copy your files in this `/media/virtuelram` directory

- test with **sha256sum**
```
IFS=$'\n'; set -f;for f in $(find /media/virtuelram -type f); do sha256sum "$f"; done;unset IFS; set +f
```

- test with **blakediff**
```
blakediff generate /media/virtuelram
```

unmount your ramdisk after use :
```
sudo umount /media/virtuelram
```