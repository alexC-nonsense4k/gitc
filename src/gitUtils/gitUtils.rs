use serde::{Serialize, Deserialize};
use serde::Serializer;
use bincode::{serialize, deserialize};
use std::io::{Bytes, Read, Write};
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;


#[repr(C)]
#[derive(Debug,Serialize,Deserialize,PartialEq)]
pub struct Branch
{
    pub name:String,
    pub commitpointer:Option<Rc<RefCell<Commit>>>,
    pub references:Rc<RefCell<References>>,
}

impl Branch {
    pub fn new(name:String)->Self
    {
        Branch{
            name:name,
            commitpointer:Some(Rc::new(RefCell::new(Commit::new()))),
            references:Rc::new(RefCell::new(References::new()))
        }
    }
}


#[repr(C)]
#[derive(Debug,Serialize,Deserialize,PartialEq)]
pub struct HEAD
{
    pub currentBranchName:String,
    pub branch:BTreeMap<String,Rc<RefCell<Branch>>>,
}

impl HEAD {
    pub fn new()->Self
    {
        HEAD
        {
            currentBranchName:String::from(""),
            branch:BTreeMap::new(),
        }
    }
    pub fn addbranch(&mut self,branchname:String)
    {
        if self.branch.contains_key(&branchname)==false
        {
            let mut newbranch=Branch::new(branchname.clone());

            for (key,value) in self.branch.get(&self.currentBranchName).cloned().unwrap().borrow().references.borrow().refermap.clone()
            {
                newbranch.references.borrow_mut().refermap.insert(key,value);
            }
            newbranch.commitpointer=self.branch.get(&self.currentBranchName).cloned().unwrap().borrow().commitpointer.clone();
            self.branch.insert(branchname.clone(),Rc::new(RefCell::new(newbranch)));
        }
        else {
            println!("This name is contained.Try another name");
        }

    }
    pub fn checkout(&mut self,branchname:String)
    {
        if self.branch.contains_key(&branchname)
        {
            self.currentBranchName=branchname;
        }
        else {
            println!("uncontained name!");
        }
    }

    pub fn showAllBranch(&self)
    {
        for (key,value) in self.branch.iter()
        {
            println!("{:?}",key);
        }
    }

}

#[repr(C)]
#[derive(Debug,Serialize,Deserialize,PartialEq)]
pub enum objecttype
{
    commit,
    tree,
    blob
}


#[repr(C)]
#[derive(Debug,Serialize,Deserialize,Clone,PartialEq)]
pub struct References
{
    pub refermap:BTreeMap<String,String>,
}

impl References {
    pub fn new()->Self
    {
        References
        {
            refermap:BTreeMap::new(),
        }
    }
    pub fn update_reference(&mut self,name:String,id:String)
    {
        self.refermap.insert(name,id);
    }
    pub fn read_reference(&self,name:String)->String
    {
        let id=String::from(self.refermap.get(&name).unwrap());
        return id;
    }
}
#[repr(C)]
#[derive(Debug,Serialize,Deserialize,PartialEq)]
pub struct Objects
{
    pub treemap:BTreeMap<String,Rc<RefCell<tree>>>,
    pub commitmap:BTreeMap<String,Rc<RefCell<Commit>>>,
    pub blobmap:BTreeMap<String,Rc<RefCell<blob>>>,
}

impl Objects {
    pub fn new()->Self
    {
        Objects
        {
            treemap:BTreeMap::new(),
            commitmap:BTreeMap::new(),
            blobmap:BTreeMap::new(),
        }
    }

}
#[repr(C)]
#[derive(Debug,Serialize,Deserialize,PartialEq)]
pub struct Commit
{
    pub parents:Vec<Option<Rc<RefCell<Commit>>>>,
    pub mergeparents:Vec<Option<Rc<RefCell<Commit>>>>,
    pub author:String ,
    pub message:String,
    pub snapshot:Rc<RefCell<tree>>,
    pub t:objecttype,
}

impl Commit {
    pub fn new()->Self
    {
        Commit
        {
            parents:Vec::new(),
            mergeparents:Vec::new(),
            author:String::new(),
            message:String::new(),
            snapshot:Rc::new(RefCell::new(tree::new(String::from(".")))),
            t:objecttype::commit,

        }
    }
}
#[repr(C)]
#[derive(Debug,Serialize,Deserialize,PartialEq)]
pub struct tree
{
    pub name:String,
    pub trees:BTreeMap<String,Rc<RefCell<tree>>>,
    pub blobs:BTreeMap<String,Rc<RefCell<blob>>>,
    pub t:objecttype,
}

impl tree {
    pub fn new(name:String)->Self
    {
        tree
        {   name:String::from(name),
            trees:BTreeMap::new(),
            blobs:BTreeMap::new(),
            t:objecttype::tree,
        }
    }
}
#[repr(C)]
#[derive(Debug,Serialize,Deserialize,PartialEq)]
pub struct blob
{
    pub name:String,
    pub contents:Vec<u8>,
    pub t:objecttype,
}

impl  blob {
    pub fn new(name:String)->Self
    {
        blob
        {
            name:String::from(name),
            contents:vec![],
            t:objecttype::blob,
        }
    }
    pub fn getContents(&mut self,path:String)
    {
        let mut file_open = fs::File::open(path).unwrap();
        file_open.read_to_end(&mut self.contents);
    }

}